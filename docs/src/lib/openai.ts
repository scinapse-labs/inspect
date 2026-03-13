import {
  SYSTEM_PRECISE,
  SYSTEM_DATA,
  SYSTEM_CONCURRENCY,
  SYSTEM_CONTRACTS,
  SYSTEM_SECURITY,
  SYSTEM_TYPOS,
  SYSTEM_RUNTIME,
  SYSTEM_VALIDATE,
  PROMPT_DATA,
  PROMPT_CONCURRENCY,
  PROMPT_CONTRACTS,
  PROMPT_SECURITY,
  PROMPT_TYPOS,
  PROMPT_RUNTIME,
  PROMPT_GENERAL,
  PROMPT_VALIDATE,
  truncateDiff,
} from "./prompts";

export interface Finding {
  issue: string;
  evidence?: string;
  severity?: string;
  file?: string;
}

export interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
}

export interface ReviewOutput {
  findings: Finding[];
  usage: TokenUsage;
}

export interface TriageEntity {
  name: string;
  type: string;
  file: string;
  risk: string;
  score: string;
  classification: string;
  change_type: string;
  public_api: boolean;
  before_content?: string;
  after_content?: string;
  blast_radius?: number;
  dependent_count?: number;
  dependency_count?: number;
  dependents?: { name: string; file: string }[];
  dependencies?: { name: string; file: string }[];
}

export interface TriageResult {
  triageText: string;
  entities: TriageEntity[];
}

/** Call inspect-api /v1/triage for entity-level risk analysis. Returns both formatted text and raw entities. */
export async function fetchTriageRich(
  apiKey: string,
  apiUrl: string,
  repo: string,
  prNumber: number
): Promise<TriageResult> {
  const empty: TriageResult = { triageText: "", entities: [] };
  try {
    const resp = await fetch(`${apiUrl}/v1/triage`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ repo, pr_number: prNumber }),
    });

    if (!resp.ok) return empty;

    const data = await resp.json();
    const entities: TriageEntity[] = data.entities || [];
    if (entities.length === 0) return empty;

    const meaningful = entities
      .filter((e) => ["modified", "added"].includes(e.change_type) && e.type !== "chunk")
      .sort((a, b) => parseFloat(b.score) - parseFloat(a.score))
      .slice(0, 30);

    if (meaningful.length === 0) return { triageText: "", entities };

    const byFile: Record<string, TriageEntity[]> = {};
    for (const e of meaningful) {
      (byFile[e.file] ??= []).push(e);
    }

    const fileEntries = Object.entries(byFile).sort(
      (a, b) =>
        Math.max(...b[1].map((e) => parseFloat(e.score))) -
        Math.max(...a[1].map((e) => parseFloat(e.score)))
    );

    const lines = ["## Entity-level triage (highest-risk changes, ranked by impact):"];
    for (const [fp, ents] of fileEntries) {
      lines.push(`\n**${fp}**:`);
      for (const e of ents) {
        const pub = e.public_api ? " [PUBLIC API]" : "";
        const score = parseFloat(e.score);
        const parts = [`${e.name} (${e.type}, ${e.change_type}, ${e.classification})${pub}`];
        parts.push(`risk=${score.toFixed(2)}`);
        if (e.blast_radius && e.blast_radius > 0) parts.push(`blast=${e.blast_radius}`);
        if (e.dependent_count && e.dependent_count > 0) parts.push(`callers=${e.dependent_count}`);
        if (e.dependency_count && e.dependency_count > 0) parts.push(`deps=${e.dependency_count}`);
        lines.push(`  - ${parts.join(" ")}`);
      }
    }
    return { triageText: lines.join("\n"), entities };
  } catch {
    return empty;
  }
}

/** Build entity context for validation: only entities mentioned in candidate issues. */
export function buildValidationEntityContext(
  entities: TriageEntity[],
  candidates: Finding[]
): string {
  if (entities.length === 0 || candidates.length === 0) return "";

  const combined = candidates.map((c) => c.issue.toLowerCase()).join(" ");

  const matched = entities.filter((e) => {
    if (!e.name || e.type === "chunk") return false;
    if (!(e.before_content || e.after_content)) return false;
    return combined.includes(e.name.toLowerCase());
  });

  if (matched.length === 0) return "";

  matched.sort((a, b) => parseFloat(b.score) - parseFloat(a.score));
  const top = matched.slice(0, 10);

  const sections: string[] = [];
  let total = 0;

  for (const e of top) {
    let section = `\n**${e.name}** (${e.type}) in ${e.file}:`;
    if (e.before_content) {
      section += `\nBEFORE:\n\`\`\`\n${e.before_content.slice(0, 1000)}\n\`\`\``;
    }
    if (e.after_content) {
      section += `\nAFTER:\n\`\`\`\n${e.after_content.slice(0, 1000)}\n\`\`\``;
    }

    total += section.length;
    if (total > 20000) break;
    sections.push(section);
  }

  return sections.join("\n");
}

function stripCodeFences(text: string): string {
  const trimmed = text.trim();
  if (trimmed.startsWith("```")) {
    let content = trimmed.slice(3);
    if (content.startsWith("json")) content = content.slice(4);
    const end = content.lastIndexOf("```");
    if (end !== -1) return content.slice(0, end).trim();
    return content.trim();
  }
  return trimmed;
}

function parseIssues(text: string): Finding[] {
  const cleaned = stripCodeFences(text);
  try {
    const data = JSON.parse(cleaned);
    const issues = data.issues || [];
    return issues
      .map((item: any) => {
        if (typeof item === "string") {
          return { issue: item };
        }
        if (typeof item === "object" && item.issue) {
          return {
            issue: item.issue,
            evidence: item.evidence || undefined,
            severity: item.severity || undefined,
            file: item.file || undefined,
          };
        }
        return null;
      })
      .filter(Boolean) as Finding[];
  } catch {
    return [];
  }
}

interface LLMResult {
  content: string;
  input_tokens: number;
  output_tokens: number;
}

async function callOpenAI(
  apiKey: string,
  model: string,
  system: string,
  prompt: string,
  temperature: number,
  seed?: number
): Promise<LLMResult> {
  const body: any = {
    model,
    messages: [
      { role: "system", content: system },
      { role: "user", content: prompt },
    ],
    temperature,
  };
  if (seed !== undefined) body.seed = seed;

  const resp = await fetch("https://api.openai.com/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`OpenAI API error ${resp.status}: ${text}`);
  }

  const data = await resp.json();
  return {
    content: data.choices?.[0]?.message?.content || "",
    input_tokens: data.usage?.prompt_tokens || 0,
    output_tokens: data.usage?.completion_tokens || 0,
  };
}

/** Extract file basenames from a unified diff. */
function extractDiffFiles(diff: string): Set<string> {
  const basenames = new Set<string>();
  for (const line of diff.split("\n")) {
    if (line.startsWith("+++ b/") || line.startsWith("--- a/")) {
      const path = line.slice(6);
      if (path && path !== "/dev/null") {
        const base = path.split("/").pop()!.toLowerCase();
        basenames.add(base);
      }
    }
  }
  return basenames;
}

const CODE_EXTS = new Set([
  ".py", ".js", ".ts", ".tsx", ".jsx", ".java", ".go", ".rs", ".rb",
  ".c", ".cpp", ".cs", ".swift", ".kt", ".scala", ".hbs", ".erb",
  ".ex", ".exs", ".hcl",
]);

/** Drop issues that mention code files not present in the diff. */
function structuralFileFilter(issues: Finding[], diffBasenames: Set<string>): Finding[] {
  if (diffBasenames.size === 0) return issues;

  return issues.filter((f) => {
    const text = f.issue.toLowerCase();
    const words = text.replace(/\//g, " / ").split(/\s+/);
    for (const word of words) {
      if ([...CODE_EXTS].some((ext) => word.endsWith(ext))) {
        const base = word.split("/").pop()!;
        if (!diffBasenames.has(base)) return false;
      }
    }
    return true;
  });
}

function fillPrompt(
  template: string,
  prTitle: string,
  diff: string,
  triage: string
): string {
  return template
    .replace("{pr_title}", prTitle)
    .replace("{triage}", triage)
    .replace("{diff}", diff);
}

/** v29: Enriched triage (risk scores, blast radius, dep counts) + 80K diff + standard validation with entity verification. */
export async function reviewV29(
  apiKey: string,
  model: string,
  prTitle: string,
  diff: string,
  triageResult: TriageResult
): Promise<ReviewOutput> {
  const truncated = truncateDiff(diff, 80000);
  const diffBasenames = extractDiffFiles(diff);

  let totalInput = 0;
  let totalOutput = 0;

  // Build all prompts with enriched triage (no entity context in lenses)
  const pData = fillPrompt(PROMPT_DATA, prTitle, truncated, triageResult.triageText);
  const pConcurrency = fillPrompt(PROMPT_CONCURRENCY, prTitle, truncated, triageResult.triageText);
  const pContracts = fillPrompt(PROMPT_CONTRACTS, prTitle, truncated, triageResult.triageText);
  const pSecurity = fillPrompt(PROMPT_SECURITY, prTitle, truncated, triageResult.triageText);
  const pTypos = fillPrompt(PROMPT_TYPOS, prTitle, truncated, triageResult.triageText);
  const pRuntime = fillPrompt(PROMPT_RUNTIME, prTitle, truncated, triageResult.triageText);
  const pGeneral = fillPrompt(PROMPT_GENERAL, prTitle, truncated, triageResult.triageText);

  // 9 lenses: 6 specialized@T=0 + 1 general@T=0 + 2 diversity@T=0.15
  const results = await Promise.allSettled([
    callOpenAI(apiKey, model, SYSTEM_DATA, pData, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_CONCURRENCY, pConcurrency, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_CONTRACTS, pContracts, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_SECURITY, pSecurity, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_TYPOS, pTypos, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_RUNTIME, pRuntime, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_PRECISE, pGeneral, 0, 42),
    callOpenAI(apiKey, model, SYSTEM_PRECISE, pGeneral, 0.15, 42),
    callOpenAI(apiKey, model, SYSTEM_PRECISE, pGeneral, 0.15, 123),
  ]);

  // Merge + dedup (first-80-char lowercase key)
  const allFindings: Finding[] = [];
  const seen = new Set<string>();

  for (const result of results) {
    if (result.status === "fulfilled") {
      totalInput += result.value.input_tokens;
      totalOutput += result.value.output_tokens;
      for (const f of parseIssues(result.value.content)) {
        const key = f.issue.toLowerCase().slice(0, 80);
        if (!seen.has(key)) {
          seen.add(key);
          allFindings.push(f);
        }
      }
    }
  }

  // Structural file filter
  const filtered = structuralFileFilter(allFindings, diffBasenames);

  const mkOutput = (findings: Finding[]): ReviewOutput => ({
    findings,
    usage: { input_tokens: totalInput, output_tokens: totalOutput },
  });

  if (filtered.length === 0) return mkOutput([]);
  if (filtered.length <= 2) return mkOutput(filtered);

  // Validation pass with entity verification context
  const candidatesText = filtered
    .map(
      (f, i) =>
        `${i + 1}. ${f.issue}${f.evidence ? `\n   Evidence: ${f.evidence}` : ""}`
    )
    .join("\n");

  const entityVerifyCtx = buildValidationEntityContext(triageResult.entities, filtered);
  const entitySection = entityVerifyCtx
    ? `\n\n## Entity Code (before/after, for verification):\n${entityVerifyCtx}\n`
    : "";

  const validatePrompt = PROMPT_VALIDATE
    .replace("{pr_title}", prTitle)
    .replace("{diff}", truncated)
    .replace("{candidates}", candidatesText)
    .replace("Candidate Issues:", `${entitySection}Candidate Issues:`);

  try {
    const validateResult = await callOpenAI(apiKey, model, SYSTEM_VALIDATE, validatePrompt, 0, 42);
    totalInput += validateResult.input_tokens;
    totalOutput += validateResult.output_tokens;
    const validated = parseIssues(validateResult.content);
    return mkOutput(validated.slice(0, 7));
  } catch {
    return mkOutput(filtered.slice(0, 5));
  }
}
