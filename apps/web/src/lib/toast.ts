import { App } from 'antd';
import type { MessageInstance } from 'antd/es/message/interface';
import type { NotificationInstance } from 'antd/es/notification/interface';
import type { HookAPI as ModalApi } from 'antd/es/modal/useModal';

let messageApi: MessageInstance | null = null;
let notificationApi: NotificationInstance | null = null;
let modalApi: ModalApi | null = null;

export function useBindAntdApi() {
  const { message, notification, modal } = App.useApp();
  messageApi = message;
  notificationApi = notification;
  modalApi = modal;
}

export const toast = {
  success: (content: string) => messageApi?.success(content),
  error: (content: string) => messageApi?.error(content),
  info: (content: string) => messageApi?.info(content),
  warning: (content: string) => messageApi?.warning(content),
  loading: (content: string, duration = 0) => messageApi?.loading(content, duration),
};

export const notify = {
  success: (message: string, description?: string) =>
    notificationApi?.success({ message, description }),
  error: (message: string, description?: string) =>
    notificationApi?.error({ message, description }),
  info: (message: string, description?: string) =>
    notificationApi?.info({ message, description }),
  warning: (message: string, description?: string) =>
    notificationApi?.warning({ message, description }),
};

export function getModalApi(): ModalApi | null {
  return modalApi;
}
