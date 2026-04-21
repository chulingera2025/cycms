import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

import common from '@/locales/zh-CN/common.json';
import auth from '@/locales/zh-CN/auth.json';
import admin from '@/locales/zh-CN/admin.json';
import pub from '@/locales/zh-CN/public.json';

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: 'zh-CN',
    supportedLngs: ['zh-CN'],
    defaultNS: 'common',
    ns: ['common', 'auth', 'admin', 'public'],
    resources: {
      'zh-CN': { common, auth, admin, public: pub },
    },
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: 'cycms_locale',
    },
  });

export default i18n;
