import './bootstrap';
import { MediaWorkspace, type MediaWorkspaceProps } from '@/features/media/MediaWorkspace';
import type { HostIslandMountContext } from '@/features/islands/runtime';
import { mountReactIsland, unmountReactIsland } from './react-island';

function readProps(value: unknown): Partial<MediaWorkspaceProps> {
  if (!value || typeof value !== 'object') {
    return {};
  }

  return value as Partial<MediaWorkspaceProps>;
}

function mediaWorkspacePropsFromContext(context: HostIslandMountContext): MediaWorkspaceProps {
  const props = readProps(context.props);

  return {
    pageTitle: typeof props.pageTitle === 'string' ? props.pageTitle : '媒体管理',
    pageDescription:
      typeof props.pageDescription === 'string' ? props.pageDescription : undefined,
  };
}

export async function mount(context: HostIslandMountContext) {
  return mountReactIsland(context, (currentContext) => (
    <MediaWorkspace {...mediaWorkspacePropsFromContext(currentContext)} />
  ));
}

export async function unmount(context: HostIslandMountContext) {
  unmountReactIsland(context.container);
}