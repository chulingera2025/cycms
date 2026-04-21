import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ModuleHostBoundary } from './ModuleHostBoundary';

function Boom(): null {
  throw new Error('boom');
}

describe('ModuleHostBoundary', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders fallback when child throws and resets on resetKey change', () => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { rerender } = render(
      <ModuleHostBoundary resetKey="first">
        <Boom />
      </ModuleHostBoundary>,
    );

    expect(screen.getByText('插件模块渲染失败')).toBeInTheDocument();
    expect(screen.getByText('boom')).toBeInTheDocument();

    rerender(
      <ModuleHostBoundary resetKey="second">
        <div>recovered</div>
      </ModuleHostBoundary>,
    );

    expect(screen.getByText('recovered')).toBeInTheDocument();
  });
});