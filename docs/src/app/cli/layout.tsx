export default function CliLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        background: "var(--bg)",
        color: "var(--fg)",
      }}
    >
      <div style={{ textAlign: "center", maxWidth: 400 }}>
        <div
          style={{
            fontSize: 24,
            fontWeight: 700,
            color: "var(--accent)",
            letterSpacing: "-0.5px",
            marginBottom: 32,
          }}
        >
          inspect
        </div>
        {children}
      </div>
    </div>
  );
}
