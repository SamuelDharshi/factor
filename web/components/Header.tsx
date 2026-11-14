"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { FactorMark } from "./Brand";
import { useWallet } from "@/lib/wallet";

const NAV = [
  { href: "/#how", label: "how_it_works" },
  { href: "/agents", label: "agents" },
  { href: "/invoices", label: "invoices" },
  { href: "/live", label: "live" },
];

export default function Header() {
  const { account, connect, disconnect, ready } = useWallet();
  const path = usePathname();
  const [scrolled, setScrolled] = useState(false);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const on = () => setScrolled(window.scrollY > 80);
    on();
    window.addEventListener("scroll", on, { passive: true });
    return () => window.removeEventListener("scroll", on);
  }, []);
  const solid = scrolled || path !== "/";
  return (
    <nav className={`nav ${solid ? "scrolled" : ""}`} aria-label="Primary">
      <div className="nav-bar flex items-center gap-5 py-2.5">
        <Link href="/" className="brand" aria-label="FACTOR on NEAR, home">
          <FactorMark size={22} />
          FACTOR
          <span className="tag">testnet</span>
        </Link>
        <div className="ml-auto hidden items-center gap-6 font-mono text-[13px] lg:flex">
          {NAV.map((n) => <Link key={n.href} href={n.href} className={`nav-link underline-draw py-1 ${path === n.href ? "" : "text-[#333] hover:text-[var(--ink)]"}`}>{n.label}</Link>)}
          <a className="nav-link underline-draw py-1 text-[#333] hover:text-[var(--ink)]" href="https://github.com/SamuelDharshi/factor" target="_blank" rel="noreferrer">github</a>
          <Link href="/agents/register" className="nav-link btn-ghost !min-h-[38px] !px-4 !text-[13px]">register_agent</Link>
        </div>
        <div className="flex items-center gap-2 lg:ml-0">
          {account ? (
            <>
              <span className="mono hidden max-w-[10rem] truncate text-xs muted sm:inline">{account}</span>
              <button className="btn-ghost !min-h-[38px] !px-4" onClick={disconnect}>Disconnect</button>
            </>
          ) : (
            <button className="btn !min-h-[38px] !px-4" disabled={!ready} onClick={connect}>Connect wallet</button>
          )}
          <button className="btn-ghost !min-h-[38px] !px-3 lg:hidden" aria-label="Menu" aria-expanded={open} onClick={() => setOpen((o) => !o)}>☰</button>
        </div>
      </div>
      {open && (
        <div className="nav-bar mt-2 flex flex-col border py-2 font-mono text-sm lg:hidden" style={{ borderColor: "var(--rule)", background: "var(--bg)" }}>
          {[...NAV, { href: "/agents/register", label: "register_agent" }].map((n) => (
            <Link key={n.href} href={n.href} onClick={() => setOpen(false)} className="min-h-[44px] py-3">{n.label}</Link>
          ))}
        </div>
      )}
    </nav>
  );
}
