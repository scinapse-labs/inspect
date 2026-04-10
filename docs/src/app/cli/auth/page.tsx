"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useState, Suspense } from "react";

function AuthFlow() {
  const searchParams = useSearchParams();
  const port = searchParams.get("port");
  const [status, setStatus] = useState<"loading" | "error">("loading");
  const [errorMsg, setErrorMsg] = useState("");

  useEffect(() => {
    if (!port) {
      setStatus("error");
      setErrorMsg("Missing port parameter. Run `inspect login` from your terminal.");
      return;
    }

    async function createKeyAndRedirect() {
      try {
        const res = await fetch("/api/keys", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: "CLI (auto)" }),
        });

        if (!res.ok) {
          const data = await res.json().catch(() => ({}));
          setStatus("error");
          setErrorMsg(data.error || `Failed to create key (${res.status})`);
          return;
        }

        const data = await res.json();
        window.location.href = `http://localhost:${port}/callback?key=${data.key}`;
      } catch {
        setStatus("error");
        setErrorMsg("Something went wrong. Try running `inspect login` again.");
      }
    }

    createKeyAndRedirect();
  }, [port]);

  if (status === "error") {
    return (
      <div>
        <div style={{ fontSize: 16, color: "var(--red)", marginBottom: 8 }}>
          Login failed
        </div>
        <div style={{ fontSize: 13, color: "var(--dim)" }}>{errorMsg}</div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ fontSize: 16, marginBottom: 8 }}>
        Authorizing CLI...
      </div>
      <div style={{ fontSize: 13, color: "var(--dim)" }}>
        Creating API key and redirecting back to your terminal.
      </div>
    </div>
  );
}

export default function CliAuthPage() {
  return (
    <Suspense
      fallback={
        <div style={{ fontSize: 16 }}>Loading...</div>
      }
    >
      <AuthFlow />
    </Suspense>
  );
}
