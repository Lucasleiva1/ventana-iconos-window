import { useState, type ReactNode, type SyntheticEvent } from "react";

interface CollapsibleSectionProps {
  id?: string;
  title: string;
  description?: string;
  meta?: string;
  initialOpen?: boolean;
  children: ReactNode;
}

export function CollapsibleSection({
  id,
  title,
  description,
  meta,
  initialOpen = false,
  children,
}: CollapsibleSectionProps) {
  const [open, setOpen] = useState(initialOpen);

  function syncOpen(event: SyntheticEvent<HTMLDetailsElement>) {
    setOpen(event.currentTarget.open);
  }

  return (
    <details id={id} className="collapsible-section" open={open} onToggle={syncOpen}>
      <summary className="collapsible-summary">
        <span className="collapsible-chevron" aria-hidden="true" />
        <span className="collapsible-title-group">
          <strong>{title}</strong>
          {description && <small>{description}</small>}
        </span>
        {meta && <span className="collapsible-meta">{meta}</span>}
      </summary>
      <div className="collapsible-content">{children}</div>
    </details>
  );
}
