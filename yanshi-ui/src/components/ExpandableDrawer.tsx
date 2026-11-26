import React, { useEffect, useId, useRef, useState } from 'react';

export type DrawerSide = 'right' | 'left' | 'center';
export type DrawerSize = 'drawer' | 'full';

export interface ExpandableDrawerProps {
  open: boolean;
  onClose: () => void;
  title: string;
  subtitle?: React.ReactNode;
  badge?: React.ReactNode;
  side?: DrawerSide;
  defaultSize?: DrawerSize;
  allowResize?: boolean;
  headerActions?: React.ReactNode;
  footer?: React.ReactNode;
  ariaLabel?: string;
  testId?: string;
  bodyClassName?: string;
  children: React.ReactNode;
}

/**
 * Native `<dialog>` top-layer slide-over drawer with one-click expansion
 * between standard drawer width (`min(780px, 88vw)`) and full-focus width
 * (`min(1320px, 96vw)`).
 *
 * Follows modern web dialog guidelines:
 * - Uses `showModal()` (with jsdom feature guard) for automatic top-layer
 *   stacking, inert background, focus trapping, and focus restoration.
 * - Supports `closedby="any"` plus Safari/WKWebView pointer-bounds light-dismiss
 *   that ignores text-selection drags ending outside the panel.
 * - Avoids redundant `role="dialog"` / `aria-modal` on native `<dialog>`.
 */
export function ExpandableDrawer({
  open,
  onClose,
  title,
  subtitle,
  badge,
  side = 'right',
  defaultSize = 'drawer',
  allowResize = true,
  headerActions,
  footer,
  ariaLabel,
  testId,
  bodyClassName,
  children,
}: ExpandableDrawerProps): React.ReactElement | null {
  const dialogRef = useRef<HTMLDialogElement | null>(null);
  const pointerDownOutsideRef = useRef(false);
  const titleId = useId();
  const [size, setSize] = useState<DrawerSize>(defaultSize);

  useEffect(() => {
    if (open) {
      setSize(defaultSize);
    }
  }, [open, defaultSize]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (open) {
      if (!dialog.open) {
        if (typeof dialog.showModal === 'function') {
          try {
            dialog.showModal();
          } catch {
            dialog.setAttribute('open', '');
          }
        } else {
          dialog.setAttribute('open', '');
        }
      }
    } else if (dialog.open) {
      dialog.returnValue = '';
      if (typeof dialog.close === 'function') {
        dialog.close();
      } else {
        dialog.removeAttribute('open');
      }
    }
  }, [open]);

  // Keep React state in sync when the browser handles Escape / `closedby="any"`.
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    const handleCancel = (ev: Event) => {
      ev.preventDefault();
      onClose();
    };
    const handleClose = () => {
      dialog.returnValue = '';
      if (open) {
        onClose();
      }
    };
    dialog.addEventListener('cancel', handleCancel);
    dialog.addEventListener('close', handleClose);
    return () => {
      dialog.removeEventListener('cancel', handleCancel);
      dialog.removeEventListener('close', handleClose);
    };
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  const isOutsideDialog = (ev: React.PointerEvent<HTMLDialogElement>): boolean => {
    const dialog = dialogRef.current;
    if (!dialog) return false;
    const rect = dialog.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) {
      return ev.target === dialog;
    }
    return (
      ev.clientX < rect.left ||
      ev.clientX > rect.right ||
      ev.clientY < rect.top ||
      ev.clientY > rect.bottom
    );
  };

  const handlePointerDown = (ev: React.PointerEvent<HTMLDialogElement>) => {
    pointerDownOutsideRef.current = ev.target === dialogRef.current && isOutsideDialog(ev);
  };

  const handlePointerUp = (ev: React.PointerEvent<HTMLDialogElement>) => {
    const startedOutside = pointerDownOutsideRef.current;
    pointerDownOutsideRef.current = false;
    if (startedOutside && ev.target === dialogRef.current && isOutsideDialog(ev)) {
      onClose();
    }
  };

  const handleKeyDown = (ev: React.KeyboardEvent<HTMLDialogElement>) => {
    if (ev.key === 'Escape') {
      ev.preventDefault();
      onClose();
    }
  };

  const dialogClasses = [
    'ys-drawer',
    `ys-drawer--${side}`,
    `ys-drawer--${size}`,
  ].join(' ');

  return (
    <dialog
      ref={dialogRef}
      className={dialogClasses}
      data-testid={testId ?? 'expandable-drawer'}
      data-side={side}
      data-size={size}
      aria-label={ariaLabel}
      aria-labelledby={ariaLabel ? undefined : titleId}
      // React passes custom string attributes through to the DOM element.
      {...({ closedby: 'any' } as Record<string, string>)}
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onKeyDown={handleKeyDown}
    >
      <div className="ys-drawer__shell">
        <header className="ys-drawer__header">
          <div className="ys-drawer__title-wrap">
            <div className="ys-drawer__title-row">
              <h2 id={titleId} className="ys-drawer__title">
                {title}
              </h2>
              {badge ? <span className="ys-drawer__badge">{badge}</span> : null}
            </div>
            {subtitle ? <div className="ys-drawer__subtitle">{subtitle}</div> : null}
          </div>

          <div className="ys-drawer__actions">
            {headerActions}
            {allowResize ? (
              <button
                type="button"
                data-testid="drawer-toggle-size"
                className="studio-btn-ghost studio-btn-sm"
                onClick={() => setSize((prev) => (prev === 'drawer' ? 'full' : 'drawer'))}
                title={size === 'drawer' ? '切换为全屏专注视图' : '恢复标准抽屉宽度'}
              >
                {size === 'drawer' ? '⤢ 全屏专注' : '⤡ 恢复抽屉'}
              </button>
            ) : null}
            <button
              type="button"
              data-testid="drawer-close-btn"
              className="studio-btn-ghost studio-btn-sm"
              onClick={onClose}
              aria-label="关闭抽屉"
            >
              ✕ 关闭 (Esc)
            </button>
          </div>
        </header>

        <div className={`ys-drawer__body ${bodyClassName ?? ''}`}>{children}</div>

        {footer ? <footer className="ys-drawer__footer">{footer}</footer> : null}
      </div>
    </dialog>
  );
}
