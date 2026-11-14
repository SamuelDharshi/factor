"use client";
// "How it works": the section pins to the screen and scroll progress steps through the four stages.
import { AnimatePresence, motion, useMotionValueEvent, useScroll, useSpring } from "motion/react";
import { useRef, useState } from "react";
import { IconCheck, IconSwap, IconVault, IconVote } from "./Icons";

const STEPS = [
  { title: "Create", text: "Invoice a DAO. We mint the NFT and its keyless vault.", code: "create_invoice(payer_dao, amount, due_ts)", Icon: IconVault },
  { title: "Accept", text: "The DAO votes. Two of three approvals make it a debt.", code: "sputnik → accept_invoice(id)", Icon: IconVote },
  { title: "Sell", text: "List it at a discount. People or agents buy it.", code: "list(id, price) → buy(id)", Icon: IconSwap },
  { title: "Settle", text: "The DAO pays the vault. Only the NFT owner withdraws.", code: "ft_transfer_call(vault) → withdraw(id)", Icon: IconCheck },
] as const;

export default function HowScroll() {
  const ref = useRef<HTMLElement>(null);
  const [active, setActive] = useState(0);
  const { scrollYProgress } = useScroll({ target: ref, offset: ["start start", "end end"] });
  useMotionValueEvent(scrollYProgress, "change", (v) => setActive(Math.min(STEPS.length - 1, Math.floor(v * STEPS.length * 0.999))));
  const fill = useSpring(scrollYProgress, { stiffness: 110, damping: 28 });
  const step = STEPS[active];

  return (
    <section id="how" ref={ref} className="relative" style={{ height: `${STEPS.length * 80 + 40}vh` }} aria-label="How it works">
      <div className="sticky top-0 flex h-screen items-center">
        <div className="wrap grid w-full items-center gap-10 md:grid-cols-[1fr_1.15fr]">
          <div>
            <p className="eyebrow">how_it_works</p>
            <h2 className="display mt-3 text-4xl font-semibold leading-[1.05] sm:text-5xl">Four steps</h2>
            <div className="relative mt-8 pl-7">
              <div className="absolute bottom-1 left-[5px] top-1 w-px" style={{ background: "var(--rule)" }} />
              <motion.div className="absolute left-[5px] top-1 w-px origin-top" style={{ background: "var(--ink)", scaleY: fill, bottom: "0.25rem" }} />
              <ol className="space-y-5">
                {STEPS.map((s, n) => (
                  <li key={s.title} className="relative">
                    <span className="absolute -left-7 top-1.5 h-[11px] w-[11px] rounded-full border-2 transition-colors duration-300"
                      style={{ borderColor: "var(--ink)", background: n <= active ? "var(--accent)" : "var(--bg)" }} />
                    <p className="flex items-baseline gap-3 transition-opacity duration-300" style={{ opacity: n === active ? 1 : 0.4 }}>
                      <span className="mono text-xs">0{n + 1}</span>
                      <span className="display text-xl font-semibold">{s.title}</span>
                    </p>
                  </li>
                ))}
              </ol>
            </div>
          </div>

          <div className="card min-h-[18rem] p-7">
            <AnimatePresence mode="wait">
              <motion.div key={step.title} initial={{ opacity: 0, y: 18 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -12 }} transition={{ duration: 0.3, ease: [0.22, 1, 0.36, 1] }}>
                <span className="grid h-14 w-14 place-items-center rounded-2xl" style={{ background: "rgba(0,236,151,0.2)", color: "#07583a" }}><step.Icon size={28} /></span>
                <p className="mono mt-6 text-xs muted">STEP 0{active + 1} / 04</p>
                <h3 className="display mt-1 text-3xl font-semibold">{step.title}</h3>
                <p className="mt-3 max-w-md text-lg muted">{step.text}</p>
                <code className="mono mt-6 block overflow-x-auto rounded-xl px-4 py-3 text-sm" style={{ background: "var(--dark)", color: "#9ff5d3" }}>{step.code}</code>
              </motion.div>
            </AnimatePresence>
          </div>
        </div>
      </div>
    </section>
  );
}
