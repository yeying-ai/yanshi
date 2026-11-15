import React from 'react';
import {
  navigateTo,
  normalizeUiTarget,
  shouldInterceptLinkClick,
} from '../lib/router';

export interface SpaLinkProps
  extends Omit<React.AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  to: string;
  replace?: boolean;
}

/**
 * Client-side navigation anchor that prevents full-page reloads on normal clicks
 * while preserving Cmd/Ctrl+Click new-tab behavior and normalizing bare `/ui` -> `/ui/`.
 */
export function SpaLink({
  to,
  replace = false,
  onClick,
  target,
  children,
  ...rest
}: SpaLinkProps) {
  const href = normalizeUiTarget(to);

  const handleClick = (event: React.MouseEvent<HTMLAnchorElement>) => {
    onClick?.(event);
    if (!shouldInterceptLinkClick(event, target)) {
      return;
    }
    event.preventDefault();
    navigateTo(href, { replace });
  };

  return (
    <a href={href} target={target} onClick={handleClick} {...rest}>
      {children}
    </a>
  );
}

export type SubpageNavSection =
  | 'console'
  | 'runs-new'
  | 'compare'
  | 'harness'
  | 'diagnostics';

export interface StudioSubpageHeaderProps {
  eyebrow: string;
  title: string;
  subtitle?: string;
  activeSection?: SubpageNavSection;
  actions?: React.ReactNode;
}

const SUBPAGE_LINKS: ReadonlyArray<{
  id: Exclude<SubpageNavSection, 'compare'>;
  label: string;
  to: string;
}> = [
  { id: 'console', label: '经典事件台', to: '/ui/console' },
  { id: 'runs-new', label: '新建运行', to: '/ui/runs/new' },
  { id: 'harness', label: 'Harness 会话', to: '/ui/harness/sessions' },
  { id: 'diagnostics', label: '系统诊断', to: '/ui/diagnostics' },
];

/**
 * Unified top navigation bar for all auxiliary subpages (`/ui/harness/*`, `/ui/runs/new`,
 * `/ui/console`, `/ui/diagnostics`, `/ui/runs/:id/compare`) so Web and Tauri Desktop users
 * always have a one-click return path to Yanshi Studio and sibling views.
 */
export function StudioSubpageHeader({
  eyebrow,
  title,
  subtitle,
  activeSection,
  actions,
}: StudioSubpageHeaderProps) {
  return (
    <header className="studio-subpage-header">
      <div className="studio-subpage-header__lead">
        <SpaLink
          to="/ui/"
          className="studio-subpage-back"
          data-testid="subpage-back-studio"
          title="返回 Yanshi Studio 可视化工作台"
        >
          ← Studio 工作台
        </SpaLink>
        <div className="studio-subpage-header__titles">
          <p className="eyebrow">{eyebrow}</p>
          <h1>{title}</h1>
          {subtitle ? <p className="studio-subpage-subtitle">{subtitle}</p> : null}
        </div>
      </div>

      <div className="studio-subpage-header__right">
        <nav className="studio-subpage-nav" aria-label="控制台全局导航">
          {SUBPAGE_LINKS.map((item) => {
            const isActive = activeSection === item.id;
            return (
              <SpaLink
                key={item.id}
                to={item.to}
                className={`studio-subpage-nav__link ${
                  isActive ? 'studio-subpage-nav__link--active' : ''
                }`}
                aria-current={isActive ? 'page' : undefined}
              >
                {item.label}
              </SpaLink>
            );
          })}
        </nav>
        {actions ? <div className="studio-subpage-header__actions">{actions}</div> : null}
      </div>
    </header>
  );
}
