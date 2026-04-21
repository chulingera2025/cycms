import { useMemo } from 'react';
import { Checkbox, Collapse, Tag } from 'antd';
import type { Permission } from '@/types';

interface Props {
  permissions: Permission[];
  value: string[];
  onChange: (value: string[]) => void;
  disabled?: boolean;
}

export function PermissionMatrix({ permissions, value, onChange, disabled }: Props) {
  const grouped = useMemo(() => {
    return permissions.reduce<Record<string, Permission[]>>((acc, p) => {
      const domain = p.domain || 'other';
      (acc[domain] ??= []).push(p);
      return acc;
    }, {});
  }, [permissions]);

  function toggleGroup(domain: string, checked: boolean) {
    const ids = grouped[domain]?.map((p) => p.id) ?? [];
    if (checked) {
      onChange([...new Set([...value, ...ids])]);
    } else {
      onChange(value.filter((id) => !ids.includes(id)));
    }
  }

  return (
    <Collapse
      defaultActiveKey={Object.keys(grouped)}
      items={Object.entries(grouped).map(([domain, perms]) => {
        const domainIds = perms.map((p) => p.id);
        const selectedCount = domainIds.filter((id) => value.includes(id)).length;
        const allChecked = selectedCount === domainIds.length && selectedCount > 0;
        const indeterminate = selectedCount > 0 && selectedCount < domainIds.length;
        return {
          key: domain,
          label: (
            <div className="flex items-center gap-2">
              <Checkbox
                checked={allChecked}
                indeterminate={indeterminate}
                disabled={disabled}
                onClick={(e) => e.stopPropagation()}
                onChange={(e) => toggleGroup(domain, e.target.checked)}
              />
              <span className="font-medium text-text">{domain}</span>
              <Tag>
                {selectedCount} / {domainIds.length}
              </Tag>
            </div>
          ),
          children: (
            <Checkbox.Group
              value={value.filter((id) => domainIds.includes(id))}
              onChange={(next) => {
                const others = value.filter((id) => !domainIds.includes(id));
                onChange([...others, ...(next as string[])]);
              }}
              disabled={disabled}
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
                gap: 8,
              }}
            >
              {perms.map((p) => (
                <Checkbox key={p.id} value={p.id}>
                  <span className="font-mono text-xs text-text">
                    {p.resource}.{p.action}
                  </span>
                  {p.scope === 'own' && (
                    <Tag className="ml-2" color="blue">
                      own
                    </Tag>
                  )}
                </Checkbox>
              ))}
            </Checkbox.Group>
          ),
        };
      })}
    />
  );
}
