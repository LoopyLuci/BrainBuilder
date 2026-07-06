import { ReactNode } from 'react';

export function Panel({
  title,
  action,
  subtitle,
  children,
}: {
  title?: string;
  action?: ReactNode;
  subtitle?: string;
  children: ReactNode;
}) {
  return (
    <div className="bb-panel">
      {title && (
        <div className="bb-panel__header">
          <div>
            <h3 className="bb-panel__title">{title}</h3>
            {subtitle && <p className="bb-panel__subtitle">{subtitle}</p>}
          </div>
          {action}
        </div>
      )}
      <div className="bb-panel__body">{children}</div>
    </div>
  );
}
