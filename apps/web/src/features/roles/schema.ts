import { z } from 'zod';

export const roleSchema = z.object({
  name: z.string().min(1, '请输入角色名称').max(64),
  description: z.string().max(255).optional(),
  permission_ids: z.array(z.string()),
});

export type RoleFormValues = z.infer<typeof roleSchema>;
