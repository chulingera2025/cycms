import { z } from 'zod';

const baseSchema = z.object({
  username: z.string().min(1, '请输入用户名').max(64),
  email: z.string().email('邮箱格式不正确'),
  is_active: z.boolean(),
  role_ids: z.array(z.string()),
});

export const userCreateSchema = baseSchema.extend({
  password: z.string().min(8, '密码长度至少 8 位'),
});

export const userUpdateSchema = baseSchema.extend({
  password: z
    .union([z.string().min(8, '密码长度至少 8 位'), z.literal('')])
    .optional(),
});

export type UserCreateInput = z.infer<typeof userCreateSchema>;
export type UserUpdateInput = z.infer<typeof userUpdateSchema>;
