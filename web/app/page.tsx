"use client";
import Link from "next/link";
import { Lockup } from "@/components/Brand";
import HowScroll from "@/components/HowScroll";
import { IconAgent, IconVault, IconVote } from "@/components/Icons";
import Intro from "@/components/Intro";
import Reveal from "@/components/Reveal";

const CARDS = [
  { Icon: IconVault, title: "Keyless vaults", text: "Every invoice owns a NEAR account with no keys." },
  { Icon: IconVote, title: "DAO approved", text: "Only the named DAO can accept it, by vote." },
  { Icon: IconAgent, title: "Agents buy", text: "People and agents buy invoices at a discount." },
];

export default function Landing() {
  return (
    <main className="relative">
      <Intro />
      <div className="page-root">
        <header className="hero">
          <div className="hero-content">
            <div className="wrap" style={{ paddingLeft: "calc(clamp(20px,4vw,40px) + 6vw)" }}>
              <div className="hero-copy">
                <h1>Invoices that own<br /><em>their money.</em></h1>
                <Lockup size={22} />
              </div>
            </div>
          </div>
          <span className="scroll-hint" aria-hidden="true">Scroll ↓</span>
        </header>

        <div className="page-sheet">
          <div className="wrap flex flex-col items-center gap-6 py-12 text-center">
            <p className="max-w-lg text-xl muted">Get paid by DAOs today.</p>
            <div className="flex flex-wrap justify-center gap-3">
              <Link href="/agents" className="btn">Agent market</Link>
              <a href="#how" className="btn-ghost">How it works</a>
            </div>
          </div>

          <section className="wrap grid gap-5 pb-20 md:grid-cols-3">
            {CARDS.map(({ Icon, title, text }, i) => (
              <Reveal key={title} delay={i * 0.08}>
                <div className="card h-full p-6">
                  <span className="grid h-11 w-11 place-items-center rounded-xl" style={{ background: "rgba(0,236,151,0.18)", color: "#07583a" }}><Icon /></span>
                  <h3 className="display mt-5 text-xl font-semibold">{title}</h3>
                  <p className="mt-2 muted">{text}</p>
                </div>
              </Reveal>
            ))}
          </section>

          <HowScroll />

          <section className="wrap border-t py-14" style={{ borderColor: "var(--rule)" }}>
            <Reveal>
              <div className="flex flex-col items-center gap-4 text-center">
                <p className="eyebrow">built on</p>
                <a href="https://near.org" target="_blank" rel="noreferrer" aria-label="NEAR Protocol">
                  {/* eslint-disable-next-line @next/next/no-img-element */}
                  <img src="/near-wordmark.svg" alt="NEAR" height={34} style={{ height: 34, width: "auto" }} />
                </a>
              </div>
            </Reveal>
          </section>
        </div>
      </div>
    </main>
  );
}
