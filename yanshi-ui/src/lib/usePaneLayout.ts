import React, { useCallback, useEffect, useState } from 'react';
import { readStorage, writeStorage } from './storage';

export interface PaneLayoutOptions {
  storageKey: string;
  defaultLeft: number;
  defaultRight: number;
  minLeft?: number;
  maxLeft?: number;
  minRight?: number;
  maxRight?: number;
  defaultLeftCollapsed?: boolean;
  defaultRightCollapsed?: boolean;
}

interface PersistedPaneState {
  leftWidth?: number;
  rightWidth?: number;
  leftCollapsed?: boolean;
  rightCollapsed?: boolean;
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

function loadPersistedPaneState(storageKey: string): PersistedPaneState {
  const raw = readStorage(storageKey, '');
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw) as PersistedPaneState;
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

/**
 * Manages collapsible + pointer-drag-resizable left/right workspace sidebars
 * with automatic `localStorage` persistence and double-click width reset.
 */
export function usePaneLayout({
  storageKey,
  defaultLeft,
  defaultRight,
  minLeft = 240,
  maxLeft = 560,
  minRight = 280,
  maxRight = 680,
  defaultLeftCollapsed = false,
  defaultRightCollapsed = false,
}: PaneLayoutOptions) {
  const initial = loadPersistedPaneState(storageKey);

  const [leftWidth, setLeftWidth] = useState<number>(() =>
    clamp(initial.leftWidth ?? defaultLeft, minLeft, maxLeft),
  );
  const [rightWidth, setRightWidth] = useState<number>(() =>
    clamp(initial.rightWidth ?? defaultRight, minRight, maxRight),
  );
  const [leftCollapsed, setLeftCollapsed] = useState<boolean>(
    () => initial.leftCollapsed ?? defaultLeftCollapsed,
  );
  const [rightCollapsed, setRightCollapsed] = useState<boolean>(
    () => initial.rightCollapsed ?? defaultRightCollapsed,
  );

  useEffect(() => {
    writeStorage(
      storageKey,
      JSON.stringify({
        leftWidth,
        rightWidth,
        leftCollapsed,
        rightCollapsed,
      }),
    );
  }, [storageKey, leftWidth, rightWidth, leftCollapsed, rightCollapsed]);

  const toggleLeft = useCallback(() => setLeftCollapsed((v) => !v), []);
  const toggleRight = useCallback(() => setRightCollapsed((v) => !v), []);
  const resetLeft = useCallback(() => {
    setLeftCollapsed(false);
    setLeftWidth(clamp(defaultLeft, minLeft, maxLeft));
  }, [defaultLeft, minLeft, maxLeft]);
  const resetRight = useCallback(() => {
    setRightCollapsed(false);
    setRightWidth(clamp(defaultRight, minRight, maxRight));
  }, [defaultRight, minRight, maxRight]);

  const startDragLeft = useCallback(
    (ev: React.PointerEvent<HTMLDivElement>) => {
      ev.preventDefault();
      const target = ev.currentTarget;
      const startX = ev.clientX;
      const originWidth = leftWidth;
      if (typeof target.setPointerCapture === 'function') {
        try {
          target.setPointerCapture(ev.pointerId);
        } catch {
          // ignore synthetic pointer events in tests
        }
      }
      const onMove = (moveEv: PointerEvent) => {
        setLeftWidth(clamp(originWidth + (moveEv.clientX - startX), minLeft, maxLeft));
      };
      const onUp = () => {
        window.removeEventListener('pointermove', onMove);
        window.removeEventListener('pointerup', onUp);
        window.removeEventListener('pointercancel', onUp);
      };
      window.addEventListener('pointermove', onMove);
      window.addEventListener('pointerup', onUp);
      window.addEventListener('pointercancel', onUp);
    },
    [leftWidth, minLeft, maxLeft],
  );

  const startDragRight = useCallback(
    (ev: React.PointerEvent<HTMLDivElement>) => {
      ev.preventDefault();
      const target = ev.currentTarget;
      const startX = ev.clientX;
      const originWidth = rightWidth;
      if (typeof target.setPointerCapture === 'function') {
        try {
          target.setPointerCapture(ev.pointerId);
        } catch {
          // ignore synthetic pointer events in tests
        }
      }
      const onMove = (moveEv: PointerEvent) => {
        setRightWidth(clamp(originWidth - (moveEv.clientX - startX), minRight, maxRight));
      };
      const onUp = () => {
        window.removeEventListener('pointermove', onMove);
        window.removeEventListener('pointerup', onUp);
        window.removeEventListener('pointercancel', onUp);
      };
      window.addEventListener('pointermove', onMove);
      window.addEventListener('pointerup', onUp);
      window.addEventListener('pointercancel', onUp);
    },
    [rightWidth, minRight, maxRight],
  );

  return {
    leftWidth,
    rightWidth,
    leftCollapsed,
    rightCollapsed,
    setLeftWidth,
    setRightWidth,
    setLeftCollapsed,
    setRightCollapsed,
    toggleLeft,
    toggleRight,
    resetLeft,
    resetRight,
    startDragLeft,
    startDragRight,
  };
}
