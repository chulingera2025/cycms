import { Suspense, lazy } from 'react';
import { Skeleton } from 'antd';

const MDEditor = lazy(() => import('@uiw/react-md-editor'));

interface Props {
  value: string;
  onChange: (value: string) => void;
  height?: number;
}

export function RichTextEditor({ value, onChange, height = 280 }: Props) {
  return (
    <div data-color-mode="inherit">
      <Suspense fallback={<Skeleton.Input active block style={{ height }} />}>
        <MDEditor
          value={value}
          onChange={(next) => onChange(next ?? '')}
          height={height}
          preview="live"
        />
      </Suspense>
    </div>
  );
}