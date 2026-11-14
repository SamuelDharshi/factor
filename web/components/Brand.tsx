/** FACTOR logo, loaded from /public/logo.svg. */
export function FactorMark({ size = 22, className = "" }: { size?: number; className?: string }) {
  // eslint-disable-next-line @next/next/no-img-element
  return <img src="/logo.svg" width={size} height={size} alt="" aria-hidden="true" className={className} />;
}

/** Official NEAR mark, loaded from /public/near-logo.svg. */
export function NearGlyph({ size = 22, className = "" }: { size?: number; className?: string }) {
  // eslint-disable-next-line @next/next/no-img-element
  return <img src="/near-logo.svg" height={size} width={(size * 27.2) / 28} alt="" aria-hidden="true" className={className} />;
}

/** [F] FACTOR  on  NEAR [N]: the lockup used in the nav and under the hero headline. */
export function Lockup({ size = 22 }: { size?: number }) {
  return (
    <div className="lockup" aria-label="FACTOR on NEAR">
      <span className="flex items-center gap-2.5"><FactorMark size={size} />FACTOR</span>
      <span className="x">on</span>
      <span className="flex items-center gap-2.5">NEAR<NearGlyph size={size} /></span>
    </div>
  );
}
