type LogoSize = "xs" | "sm" | "md" | "lg" | "xl";

const sizes: Record<LogoSize, number> = {
  xs: 18,
  sm: 28,
  md: 48,
  lg: 72,
  xl: 128
};

/** Canonical brand mark: exact `public/logo.png`, synchronized from the root `logo.png`. */
export function AlvenqisLogo({
  size = "md",
  className = "",
  framed = false,
  alt = "Alvenqis"
}: {
  size?: LogoSize;
  className?: string;
  framed?: boolean;
  alt?: string;
}) {
  const px = sizes[size];
  const img = (
    <img
      src="/logo.png"
      alt={alt}
      width={px}
      height={px}
      className={`alvenqis-logo-img ${className}`.trim()}
      draggable={false}
      decoding="async"
      style={{ background: "transparent", objectFit: "contain" }}
    />
  );

  if (!framed) return img;

  return (
    <div className={`alvenqis-logo-frame alvenqis-logo-frame-${size}`} aria-hidden={alt === ""}>
      {img}
    </div>
  );
}
