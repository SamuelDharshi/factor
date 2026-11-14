import { scoreTone } from "@/lib/near";

const TONES = {
  green: "bg-emerald-500/20 text-emerald-900",
  amber: "bg-amber-500/20 text-amber-900",
  red: "bg-red-500/15 text-red-900",
} as const;

export default function ScoreBadge({ score }: { score: number }) {
  return (
    <span title="Payer score 0-100: 60% on-time rate, 25% volume, 15% history. Green ≥75, amber 50-74, red <50."
      className={`rounded-full px-2 py-0.5 text-xs font-semibold ${TONES[scoreTone(score)]}`}>
      score {score}
    </span>
  );
}
