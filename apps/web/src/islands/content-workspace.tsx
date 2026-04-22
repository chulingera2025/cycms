import './bootstrap';
import { ContentWorkspace, type ContentWorkspaceProps } from '@/features/content/ContentWorkspace';
import type { HostIslandMountContext } from '@/features/islands/runtime';
import { mountReactIsland, unmountReactIsland } from './react-island';

type ContentWorkspaceIslandProps = Omit<ContentWorkspaceProps, 'onSelectedTypeChange'>;

function readProps(value: unknown): Partial<ContentWorkspaceIslandProps> {
  if (!value || typeof value !== 'object') {
    return {};
  }

  return value as Partial<ContentWorkspaceIslandProps>;
}

function normalizeStatusFilter(value: unknown): ContentWorkspaceIslandProps['defaultStatusFilter'] {
  return value === 'draft' || value === 'published' || value === 'archived' ? value : '';
}

function contentWorkspacePropsFromContext(
  context: HostIslandMountContext,
): ContentWorkspaceIslandProps {
  const props = readProps(context.props);

  return {
    pageTitle: typeof props.pageTitle === 'string' ? props.pageTitle : '内容管理',
    pageDescription:
      typeof props.pageDescription === 'string' ? props.pageDescription : undefined,
    fixedTypeApiId:
      typeof props.fixedTypeApiId === 'string' ? props.fixedTypeApiId : undefined,
    selectedTypeApiId:
      typeof props.selectedTypeApiId === 'string' ? props.selectedTypeApiId : undefined,
    createLabel: typeof props.createLabel === 'string' ? props.createLabel : '新建内容',
    autoOpenCreate: props.autoOpenCreate === true,
    defaultStatusFilter: normalizeStatusFilter(props.defaultStatusFilter),
    showTypeSelector: props.showTypeSelector ?? true,
    showStatusFilter: props.showStatusFilter ?? true,
    showSlugSearch: props.showSlugSearch ?? true,
    slugSearchPlaceholder:
      typeof props.slugSearchPlaceholder === 'string'
        ? props.slugSearchPlaceholder
        : '搜索 slug',
  };
}

export async function mount(context: HostIslandMountContext) {
  return mountReactIsland(context, (currentContext) => (
    <ContentWorkspace {...contentWorkspacePropsFromContext(currentContext)} />
  ));
}

export async function unmount(context: HostIslandMountContext) {
  unmountReactIsland(context.container);
}