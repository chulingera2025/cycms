import type { ReactElement } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import type { HostIslandMountContext } from '@/features/islands/runtime';
import { HostIslandShell } from './HostIslandShell';

const mountedRoots = new WeakMap<HTMLElement, Root>();

export function unmountReactIsland(container: HTMLElement) {
  const root = mountedRoots.get(container);
  if (!root) {
    return;
  }

  root.unmount();
  mountedRoots.delete(container);
  container.innerHTML = '';
}

export function mountReactIsland(
  context: HostIslandMountContext,
  render: (context: HostIslandMountContext) => ReactElement,
) {
  unmountReactIsland(context.container);

  const root = createRoot(context.container);
  mountedRoots.set(context.container, root);
  root.render(<HostIslandShell auth={context.auth}>{render(context)}</HostIslandShell>);

  return {
    unmount: () => {
      unmountReactIsland(context.container);
    },
  };
}