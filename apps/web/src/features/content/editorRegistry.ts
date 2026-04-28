import { createContext, useContext } from 'react';

export type OwnershipMode = 'replace' | 'wrap' | 'append';

export interface HostEditorEntry {
  id: string;
  pluginName: string;
  editor: string;
  contentTypes: string[];
  fieldTypes: string[];
  screenTargets: string[];
  modules: string[];
  styles: string[];
  priority: number;
  ownership: OwnershipMode;
  declarationOrder: number;
}

export const EditorRegistryCtx = createContext<HostEditorEntry[]>([]);

export function useEditorRegistry(): HostEditorEntry[] {
  return useContext(EditorRegistryCtx);
}

export function parseEditorRegistry(raw: unknown): HostEditorEntry[] {
  if (!Array.isArray(raw)) return [];
  return raw.filter(
    (entry): entry is HostEditorEntry =>
      entry !== null &&
      typeof entry === 'object' &&
      typeof (entry as HostEditorEntry).id === 'string' &&
      typeof (entry as HostEditorEntry).pluginName === 'string' &&
      typeof (entry as HostEditorEntry).editor === 'string' &&
      Array.isArray((entry as HostEditorEntry).contentTypes) &&
      Array.isArray((entry as HostEditorEntry).fieldTypes) &&
      Array.isArray((entry as HostEditorEntry).screenTargets) &&
      Array.isArray((entry as HostEditorEntry).modules) &&
      Array.isArray((entry as HostEditorEntry).styles) &&
      typeof (entry as HostEditorEntry).priority === 'number' &&
      typeof (entry as HostEditorEntry).ownership === 'string' &&
      typeof (entry as HostEditorEntry).declarationOrder === 'number',
  );
}

function matchesSelector(selectors: string[], value: string | undefined) {
  return selectors.length === 0 || (value !== undefined && selectors.includes(value));
}

function ownershipRank(ownership: OwnershipMode) {
  switch (ownership) {
    case 'replace':
      return 0;
    case 'wrap':
      return 1;
    case 'append':
      return 2;
    default:
      return 3;
  }
}

export interface EditorResolveTarget {
  contentType?: string;
  fieldType?: string;
  screenTarget?: string;
}

export function resolveEditorOverride(
  entries: HostEditorEntry[],
  target: EditorResolveTarget,
): HostEditorEntry | undefined {
  return entries
    .filter((entry) => matchesSelector(entry.contentTypes, target.contentType))
    .filter((entry) => matchesSelector(entry.fieldTypes, target.fieldType))
    .filter((entry) => matchesSelector(entry.screenTargets, target.screenTarget))
    .sort((left, right) => {
      if (right.priority !== left.priority) {
        return right.priority - left.priority;
      }
      const ownership = ownershipRank(left.ownership) - ownershipRank(right.ownership);
      if (ownership !== 0) {
        return ownership;
      }
      if (left.declarationOrder !== right.declarationOrder) {
        return left.declarationOrder - right.declarationOrder;
      }
      return left.id.localeCompare(right.id);
    })[0];
}
