import { z } from 'zod';

export const loginSchema = z.object({
  username: z.string().min(1, '请输入用户名'),
  password: z.string().min(1, '请输入密码'),
});

export type LoginInput = z.infer<typeof loginSchema>;

export const registerSchema = z
  .object({
    username: z
      .string()
      .min(3, '用户名至少 3 个字符')
      .max(32, '用户名不超过 32 个字符'),
    email: z.string().email('邮箱格式不正确'),
    password: z.string().min(8, '密码长度至少 8 位'),
    confirmPassword: z.string(),
  })
  .refine((d) => d.password === d.confirmPassword, {
    path: ['confirmPassword'],
    message: '两次输入的密码不一致',
  });

export type RegisterInput = z.infer<typeof registerSchema>;
