import { useEffect, useMemo, useState } from 'react';
import { Select, Spin } from 'antd';
import type { DefaultOptionType } from 'antd/es/select';
import { useQuery } from '@tanstack/react-query';
import { contentApi } from '@/lib/api';

interface Props {
  target: string;
  multiple?: boolean;
  value: string | string[] | null | undefined;
  onChange: (v: string | string[] | null) => void;
  disabled?: boolean;
  placeholder?: string;
}

function useDebounced<T>(value: T, delay = 300) {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), delay);
    return () => clearTimeout(t);
  }, [value, delay]);
  return v;
}

export function RelationSelect({
  target,
  multiple,
  value,
  onChange,
  disabled,
  placeholder,
}: Props) {
  const [keyword, setKeyword] = useState('');
  const debounced = useDebounced(keyword, 250);

  const params = useMemo<Record<string, string>>(() => {
    const p: Record<string, string> = { page: '1', pageSize: '20' };
    if (debounced) p['filter[slug][contains]'] = debounced;
    return p;
  }, [debounced]);

  const { data, isFetching } = useQuery({
    queryKey: ['relation-search', target, params],
    queryFn: () => contentApi.list(target, params),
    enabled: Boolean(target),
    staleTime: 15_000,
  });

  const options: DefaultOptionType[] = useMemo(() => {
    const current = Array.isArray(value) ? value : value ? [value] : [];
    const fromData = (data?.data ?? []).map((e) => ({
      value: e.id,
      label: e.slug ? `${e.slug} · ${e.id.slice(0, 6)}` : e.id,
    }));
    const missing = current
      .filter((id) => !fromData.some((o) => o.value === id))
      .map((id) => ({ value: id, label: id }));
    return [...missing, ...fromData];
  }, [data, value]);

  return (
    <Select
      mode={multiple ? 'multiple' : undefined}
      showSearch
      allowClear
      disabled={disabled || !target}
      filterOption={false}
      value={value ?? (multiple ? [] : undefined)}
      placeholder={placeholder ?? (target ? `搜索 ${target}` : '未设置关联目标')}
      notFoundContent={isFetching ? <Spin size="small" /> : null}
      options={options}
      onSearch={setKeyword}
      onChange={(v) => {
        if (multiple) {
          onChange((v as string[]) ?? []);
        } else {
          onChange((v as string) ?? null);
        }
      }}
      style={{ width: '100%' }}
    />
  );
}
