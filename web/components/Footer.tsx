import Link from "next/link";

export default function Footer() {
  return (
    <footer className="relative z-10 overflow-hidden" style={{ background: "var(--bg)" }}>
      <div className="wrap flex flex-wrap items-center justify-between gap-4 pt-10 text-sm muted">
        <span>Testnet demo. mUSDC is a mock token.</span>
        <nav className="flex gap-5 font-mono text-xs" aria-label="Footer">
          <Link className="underline-draw" href="/#how">how_it_works</Link>
          <Link className="underline-draw" href="/live">live</Link>
          <a className="underline-draw" href="https://github.com/SamuelDharshi/factor" target="_blank" rel="noreferrer">github</a>
        </nav>
      </div>
      {/* faded wordmark */}
      <div aria-hidden className="select-none whitespace-nowrap text-center leading-[0.78]"
        style={{ fontFamily: "var(--hero)", fontSize: "clamp(6rem, 24vw, 22rem)", color: "rgba(17,17,17,0.07)", letterSpacing: "-0.03em", marginTop: "1.5rem", marginBottom: "-0.12em" }}>
        FACTOR
      </div>
    </footer>
  );
}
