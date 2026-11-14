// Simple line icons (24px, currentColor) used on cards.
type P = { size?: number; className?: string };
const base = (size: number, className: string) => ({
  width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.6,
  strokeLinecap: "round" as const, strokeLinejoin: "round" as const, className, "aria-hidden": true,
});

/** padlock: keyless vault */
export const IconVault = ({ size = 22, className = "" }: P) => (
  <svg {...base(size, className)}><rect x="4" y="10" width="16" height="10" rx="2.5" /><path d="M8 10V7.5a4 4 0 0 1 8 0V10" /><circle cx="12" cy="15" r="1.3" /></svg>
);
/** ballot check: DAO vote */
export const IconVote = ({ size = 22, className = "" }: P) => (
  <svg {...base(size, className)}><rect x="3.5" y="9" width="17" height="11" rx="2.5" /><path d="M8.5 9 12 4l3.5 5" /><path d="m9.5 14.5 1.8 1.8 3.4-3.6" /></svg>
);
/** bot head: agents */
export const IconAgent = ({ size = 22, className = "" }: P) => (
  <svg {...base(size, className)}><rect x="4" y="8" width="16" height="11" rx="3" /><path d="M12 8V4.5" /><circle cx="12" cy="3.6" r="0.9" /><circle cx="9" cy="13" r="1" /><circle cx="15" cy="13" r="1" /><path d="M9.5 16.5h5" /></svg>
);
/** arrows: trade */
export const IconSwap = ({ size = 22, className = "" }: P) => (
  <svg {...base(size, className)}><path d="M4 8h14l-3-3" /><path d="M20 16H6l3 3" /></svg>
);
/** coin check: settled */
export const IconCheck = ({ size = 22, className = "" }: P) => (
  <svg {...base(size, className)}><circle cx="12" cy="12" r="8.5" /><path d="m8.5 12.3 2.4 2.4 4.6-5" /></svg>
);
