import { act, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useLocation } from 'react-router-dom';
import type { HostIslandMountContext } from '@/features/islands/runtime';
import { mountReactIsland } from './react-island';

function hostContext(container: HTMLElement): HostIslandMountContext {
  return {
    container,
    pluginName: 'host',
    contributionId: 'core.island.test',
    contributionKind: 'route',
    fullPath: '/admin/island-test',
    sdkVersion: '1.0.0',
    apiClient: {} as HostIslandMountContext['apiClient'],
    queryClient: {} as HostIslandMountContext['queryClient'],
    auth: {
      user: null,
      isAdmin: false,
      isMember: false,
      refresh: vi.fn(async () => undefined),
      logout: vi.fn(() => undefined),
    },
    navigation: {
      pathname: '/admin/island-test',
      search: '',
      hash: '',
      navigate: vi.fn(),
    },
    logger: {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
    },
    islandId: 'core:island:test',
    component: 'core:island:test',
    props: null,
    inlineData: {},
    pageMode: 'island',
  };
}

describe('react island helper', () => {
  it('mounts and unmounts a provider-backed island', async () => {
    document.body.innerHTML = '<div id="island-root"></div>';
    const container = document.getElementById('island-root');
    if (!(container instanceof HTMLElement)) {
      throw new Error('expected test container');
    }

    let handle: ReturnType<typeof mountReactIsland> | null = null;
    await act(async () => {
      handle = mountReactIsland(hostContext(container), () => <div>host island ready</div>);
    });

    expect(await screen.findByText('host island ready')).toBeInTheDocument();

    await act(async () => {
      handle?.unmount();
    });

    expect(container).toBeEmptyDOMElement();
  });

  it('provides router context for island consumers', async () => {
    window.history.replaceState(null, '', '/admin/editor-island');
    document.body.innerHTML = '<div id="router-root"></div>';
    const container = document.getElementById('router-root');
    if (!(container instanceof HTMLElement)) {
      throw new Error('expected router test container');
    }

    function LocationProbe() {
      const location = useLocation();
      return <div>{location.pathname}</div>;
    }

    let handle: ReturnType<typeof mountReactIsland> | null = null;
    await act(async () => {
      handle = mountReactIsland(hostContext(container), () => <LocationProbe />);
    });

    expect(await screen.findByText('/admin/editor-island')).toBeInTheDocument();

    await act(async () => {
      handle?.unmount();
    });
  });
});