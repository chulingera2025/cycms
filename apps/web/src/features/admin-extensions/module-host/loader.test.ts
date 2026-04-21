import { afterEach, describe, expect, it } from 'vitest';
import { loadAdminPluginModule, retainPluginStyles } from './loader';

describe('admin plugin loader', () => {
  afterEach(() => {
    document.head.querySelectorAll('link[data-cycms-plugin-style]').forEach((node) => node.remove());
  });

  it('rejects non same-origin assets before loading', async () => {
    await expect(retainPluginStyles(['https://cdn.example.com/plugin.css'])).rejects.toThrow(
      '不是同源 URL',
    );

    await expect(loadAdminPluginModule('https://cdn.example.com/plugin.js')).rejects.toThrow(
      '不是同源 URL',
    );
  });

  it('retains same-origin styles until released', async () => {
    const pending = retainPluginStyles(['/plugins/demo/style.css']);
    const link = document.head.querySelector('link[data-cycms-plugin-style]');
    expect(link).not.toBeNull();

    link?.dispatchEvent(new Event('load'));
    const release = await pending;

    expect((link as HTMLLinkElement).href).toBe('http://localhost:3000/plugins/demo/style.css');

    release();
    expect(document.head.querySelector('link[data-cycms-plugin-style]')).toBeNull();
  });
});