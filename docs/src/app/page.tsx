"use client";

import { useEffect } from "react";
import Nav from "@/components/nav";

function copyCmd(el: HTMLElement, cmd: string) {
  navigator.clipboard.writeText(cmd);
  const copied = el.querySelector(".copied") as HTMLElement | null;
  if (copied) {
    copied.classList.add("show");
    setTimeout(() => copied.classList.remove("show"), 1500);
  }
}

export default function HomePage() {
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("visible");
            const bars =
              e.target.querySelectorAll<HTMLElement>(".bench-bar[data-width]");
            bars.forEach((bar, i) => {
              setTimeout(() => {
                bar.style.width = bar.dataset.width + "%";
              }, i * 100);
            });
          }
        });
      },
      { threshold: 0.1 }
    );

    document.querySelectorAll(".fade-in").forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  return (
    <div className="container">
      <Nav active="home" />

      {/* Act 1: Hero */}
      <div style={{ padding: "60px 0 20px", textAlign: "center" }}>
        <h1
          style={{
            fontSize: 42,
            fontWeight: 700,
            color: "var(--accent)",
            letterSpacing: "-2px",
            marginBottom: 12,
          }}
        >
          Review what matters.
        </h1>
        <p
          style={{
            fontSize: 14,
            color: "var(--dim)",
            marginBottom: 32,
            lineHeight: 1.7,
            maxWidth: 500,
            marginLeft: "auto",
            marginRight: "auto",
          }}
        >
          Entity-level code review for Git. 8 files changed, but only 2 need
          careful review. inspect tells you which ones in 6ms.
        </p>
      </div>

      {/* Side-by-side: git diff vs inspect */}
      <div className="side-by-side fade-in">
        <div className="terminal">
          <div className="terminal-bar">
            <div className="terminal-dot" />
            <div className="terminal-dot" />
            <div className="terminal-dot" />
            <div className="terminal-title">git diff</div>
          </div>
          <div className="terminal-body">
            <pre
              dangerouslySetInnerHTML={{
                __html: `<span class="cmd">$ git diff --stat HEAD~1</span>
<span class="d"> src/merge/core.rs    | 47 +++++++++---</span>
<span class="d"> src/validate.rs      | 12 ------</span>
<span class="d"> src/config.rs        | 23 +++++++</span>
<span class="d"> src/display.rs       |  8 ++--</span>
<span class="d"> src/driver/mod.rs    | 15 ++++--</span>
<span class="d"> src/driver/parse.rs  |  9 ++++</span>
<span class="d"> tests/merge_test.rs  | 31 +++++++++</span>
<span class="d"> README.md            |  4 +-</span>

<span class="d"> 8 files changed, 128(+), 21(-)</span>

<span class="d"># Which files actually matter?</span>
<span class="d"># Read all of them to find out.</span>`,
              }}
            />
          </div>
        </div>
        <div className="terminal">
          <div className="terminal-bar">
            <div className="terminal-dot" />
            <div className="terminal-dot" />
            <div className="terminal-dot" />
            <div className="terminal-title">inspect</div>
          </div>
          <div className="terminal-body">
            <pre
              dangerouslySetInnerHTML={{
                __html: `<span class="cmd">$ inspect diff HEAD~1</span>

<span class="r">CRITICAL</span> <span class="w">merge_entities</span> <span class="d">(src/merge/core.rs)</span>
  <span class="d">blast: 171  deps: 12  public API</span>
  <span class="r">&gt;&gt;&gt; 12 dependents may be affected</span>

<span class="o">HIGH</span>     <span class="w">old_validate</span> <span class="d">(src/validate.rs)</span>
  <span class="d">blast: 8  public API  deleted</span>

<span class="g">6 other changes are low risk</span>
<span class="d">verdict:</span> <span class="o">requires_careful_review</span>`,
              }}
            />
          </div>
        </div>
      </div>

      {/* Install */}
      <div style={{ textAlign: "center", margin: "32px 0 40px" }}>
        <div
          className="install-box"
          onClick={(e) =>
            copyCmd(
              e.currentTarget,
              "brew install ataraxy-labs/tap/inspect"
            )
          }
          style={{
            display: "inline-block",
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 8,
            padding: "10px 20px",
            fontSize: 14,
            color: "var(--fg)",
            cursor: "pointer",
            position: "relative",
          }}
        >
          <span
            className="copied"
            style={{
              position: "absolute",
              top: -28,
              left: "50%",
              transform: "translateX(-50%)",
              fontSize: 12,
              color: "var(--green)",
              opacity: 0,
              transition: "opacity 0.2s",
            }}
          >
            copied
          </span>
          <span style={{ color: "var(--dim)" }}>$</span> brew install
          ataraxy-labs/tap/inspect
        </div>
        <div style={{ fontSize: 12, color: "var(--dim)", marginTop: 8 }}>
          or{" "}
          <code style={{ background: "var(--surface)", padding: "2px 6px", borderRadius: 3 }}>
            cargo install --git https://github.com/Ataraxy-Labs/inspect inspect-cli
          </code>
        </div>
      </div>

      {/* Act 2: The Proof */}
      <section className="fade-in">
        <h2>95% recall.</h2>
        <p className="section-desc">
          141 planted bugs, 52 PRs, 5 repos.{" "}
          <a
            href="https://huggingface.co/datasets/rs545837/inspect-greptile-bench"
            style={{ color: "var(--cyan)" }}
          >
            Dataset on HuggingFace
          </a>
          .{" "}
          <a href="/benchmarks" style={{ color: "var(--cyan)" }}>
            Full benchmarks {"\u2192"}
          </a>
        </p>

        <div className="bench-group">
          {[
            {
              name: "inspect + GPT-5.2",
              value: "95.0%",
              valueColor: "var(--green)",
              width: 95,
              cls: "inspect-bar",
              bold: true,
            },
            {
              name: "Greptile (API)",
              value: "91.5%",
              width: 91.5,
              cls: "other-bar",
              bold: false,
            },
            {
              name: "CodeRabbit (CLI)",
              value: "56.0%",
              width: 56,
              cls: "dim-bar",
              bold: false,
            },
          ].map((row, i) => (
            <div className="bench-row" key={i}>
              <div className="bench-label">
                <span className="name">
                  {row.bold ? (
                    <strong style={{ color: "var(--accent)" }}>
                      {row.name}
                    </strong>
                  ) : (
                    row.name
                  )}
                </span>
                <span
                  className="value"
                  style={
                    row.valueColor ? { color: row.valueColor } : undefined
                  }
                >
                  {row.value}
                </span>
              </div>
              <div className="bench-bar-track">
                <div
                  className={`bench-bar ${row.cls}`}
                  data-width={row.width}
                >
                  {row.value}
                </div>
              </div>
            </div>
          ))}
        </div>

        <div className="stat-pills">
          <div className="stat-pill">
            <span className="val" style={{ color: "var(--green)" }}>
              100%
            </span>
            <span className="lbl">HC recall (every critical bug caught)</span>
          </div>
          <div className="stat-pill">
            <span className="val" style={{ color: "var(--green)" }}>
              6ms
            </span>
            <span className="lbl">per commit</span>
          </div>
          <div className="stat-pill">
            <span className="val" style={{ color: "var(--accent)" }}>0</span>
            <span className="lbl">API calls needed</span>
          </div>
        </div>
      </section>

      {/* Act 3: How it works */}
      <section className="fade-in">
        <h2>Four phases</h2>
        <p className="section-desc">
          All local. No LLM, no network calls. Optionally, send the top
          entities to an LLM for full review.{" "}
          <a href="/docs" style={{ color: "var(--cyan)" }}>
            Full docs {"\u2192"}
          </a>
        </p>

        <div className="phase-cards">
          <div className="phase-card" style={{ borderColor: "var(--green)" }}>
            <div
              className="tag"
              style={{ background: "#4ade8022", color: "var(--green)" }}
            >
              EXTRACT
            </div>
            <h3>Parse</h3>
            <p>
              tree-sitter extracts entities from all tracked source files. Builds
              a full-repo dependency graph via call/reference analysis.
            </p>
          </div>
          <div className="phase-card" style={{ borderColor: "var(--cyan)" }}>
            <div
              className="tag"
              style={{ background: "#22d3ee22", color: "var(--cyan)" }}
            >
              CLASSIFY
            </div>
            <h3>Categorize</h3>
            <p>
              Compare before/after. Classify each change as text (comments),
              syntax (signatures), functional (logic), or a combination.
            </p>
          </div>
          <div className="phase-card" style={{ borderColor: "var(--yellow)" }}>
            <div
              className="tag"
              style={{ background: "#facc1522", color: "var(--yellow)" }}
            >
              SCORE
            </div>
            <h3>Risk</h3>
            <p>
              Graph-centric scoring. Dependents and blast radius are the primary
              signals. Public API and change type set the baseline.
            </p>
          </div>
          <div className="phase-card" style={{ borderColor: "var(--purple)" }}>
            <div
              className="tag"
              style={{ background: "#a78bfa22", color: "var(--purple)" }}
            >
              GROUP
            </div>
            <h3>Untangle</h3>
            <p>
              Union-Find on dependency edges. Separates independent logical
              changes within tangled commits.
            </p>
          </div>
        </div>
      </section>

      {/* Languages */}
      <section className="fade-in">
        <h2>21 languages</h2>
        <p className="section-desc">
          Entity extraction powered by{" "}
          <a
            href="https://github.com/Ataraxy-Labs/sem"
            style={{ color: "var(--green)" }}
          >
            sem-core
          </a>{" "}
          and tree-sitter. Plus 5 data formats.{" "}
          <a href="/docs" style={{ color: "var(--cyan)" }}>
            Full list {"\u2192"}
          </a>
        </p>

        <div className="lang-chips">
          {[
            "TypeScript",
            "JavaScript",
            "Python",
            "Go",
            "Rust",
            "Java",
            "C",
            "C++",
            "C#",
            "Ruby",
            "PHP",
            "Swift",
            "Kotlin",
            "Elixir",
            "Bash",
            "HCL",
            "Fortran",
            "Vue",
            "Svelte",
            "XML",
            "ERB",
          ].map((lang) => (
            <span className="lang-chip" key={lang}>
              {lang}
            </span>
          ))}
          {["JSON", "YAML", "TOML", "CSV", "Markdown"].map((fmt) => (
            <span className="lang-chip data" key={fmt}>
              {fmt}
            </span>
          ))}
        </div>
      </section>

      {/* Try it */}
      <section className="fade-in" style={{ textAlign: "center" }}>
        <h2>Try it. 5 seconds.</h2>
        <div className="try-terminal">
          <div className="terminal">
            <div className="terminal-bar">
              <div className="terminal-dot" />
              <div className="terminal-dot" />
              <div className="terminal-dot" />
              <div className="terminal-title">~/my-project</div>
            </div>
            <div className="terminal-body" style={{ textAlign: "left" }}>
              <pre
                dangerouslySetInnerHTML={{
                  __html: `<span class="cmd">$ brew install ataraxy-labs/tap/inspect</span>

<span class="cmd">$ inspect diff HEAD~1</span>
<span class="w">inspect</span> 12 entities changed
  <span class="r">1 critical</span>, <span class="o">2 high</span>, <span class="y">3 medium</span>, <span class="d">6 low</span>
<span class="d">verdict:</span> <span class="o">requires_careful_review</span>`,
                }}
              />
            </div>
          </div>
        </div>
      </section>

      <footer>
        <p>
          Built by <a href="https://ataraxy-labs.com">Ataraxy Labs</a>
        </p>
      </footer>
    </div>
  );
}
