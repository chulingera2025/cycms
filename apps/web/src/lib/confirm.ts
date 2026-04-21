import { getModalApi } from './toast';

interface ConfirmOptions {
  title: string;
  content?: string;
  okText?: string;
  cancelText?: string;
  danger?: boolean;
}

export function confirmDialog(opts: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    const modal = getModalApi();
    if (!modal) {
      resolve(false);
      return;
    }
    modal.confirm({
      title: opts.title,
      content: opts.content,
      okText: opts.okText ?? '确认',
      cancelText: opts.cancelText ?? '取消',
      okButtonProps: opts.danger ? { danger: true } : undefined,
      onOk: () => resolve(true),
      onCancel: () => resolve(false),
    });
  });
}
