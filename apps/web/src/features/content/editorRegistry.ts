import { createContext, useContext } from 'react';

export interface HostEditorEntry {
  id: string;
  editor: string;
  contentTypes: string[];
  fieldTypes: string[];
  screenTargets: string[];
  modules: string[];
  styles: string[];
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
      typeof (entry as HostEditorEntry).editor === 'string',
  );
}
