import { writable, derived } from 'svelte/store';
import en from './locales/en.json';
import zh from './locales/zh.json';
import ja from './locales/ja.json';

export type Language = 'en' | 'zh' | 'ja';
export type TranslationKey = string;

const translations = {
  en,
  zh,
  ja
};

// 从localStorage获取保存的语言设置，默认为英文
const storedLanguage = typeof window !== 'undefined' ? localStorage.getItem('cortex-mem-language') as Language : null;
const defaultLanguage: Language = storedLanguage && ['en', 'zh', 'ja'].includes(storedLanguage) ? storedLanguage : 'en';

// 创建语言store
export const language = writable<Language>(defaultLanguage);

// 创建翻译store
export const t = derived(language, ($language) => {
  const currentTranslations = translations[$language];
  
  // 创建翻译函数
  const translate = (key: TranslationKey, params?: Record<string, string | number>): string => {
    // 支持嵌套key，如 'common.appName'
    const keys = key.split('.');
    let value: any = currentTranslations;
    
    for (const k of keys) {
      if (value && typeof value === 'object' && k in value) {
        value = value[k];
      } else {
        // 如果找不到翻译，回退到英文
        let fallbackValue: any = translations.en;
        for (const fallbackKey of keys) {
          if (fallbackValue && typeof fallbackValue === 'object' && fallbackKey in fallbackValue) {
            fallbackValue = fallbackValue[fallbackKey];
          } else {
            return key; // 如果英文也没有，返回key本身
          }
        }
        value = fallbackValue;
        break;
      }
    }
    
    // 如果找到了字符串值，处理参数替换
    if (typeof value === 'string' && params) {
      return Object.entries(params).reduce((str, [paramKey, paramValue]) => {
        return str.replace(new RegExp(`\{${paramKey}\}`, 'g'), String(paramValue));
      }, value);
    }
    
    return typeof value === 'string' ? value : key;
  };
  
  return translate;
});

// 切换语言函数
export function setLanguage(newLanguage: Language): void {
  language.set(newLanguage);
  if (typeof window !== 'undefined') {
    localStorage.setItem('cortex-mem-language', newLanguage);
  }
}

// 获取当前语言
export function getCurrentLanguage(): Language {
  let currentLang: Language = 'en';
  language.subscribe((lang) => {
    currentLang = lang;
  })();
  return currentLang;
}

// 获取语言选项
export const languageOptions = [
  { value: 'en', label: 'English' },
  { value: 'zh', label: '中文' },
  { value: 'ja', label: '日本語' }
];

// 格式化函数：用于格式化数字、日期等
export const format = {
  // 格式化重要性分数（保留两位小数）
  importance: (value: number): string => {
    return value.toFixed(2);
  },
  
  // 格式化质量分数（保留两位小数）
  quality: (value: number): string => {
    return value.toFixed(2);
  },
  
  // 格式化百分比（用于进度条等）
  percentage: (value: number): string => {
    return `${(value * 100).toFixed(1)}%`;
  },
  
  // 格式化日期
  date: (dateString: string, locale?: string): string => {
    try {
      const date = new Date(dateString);
      const currentLocale = locale || getCurrentLanguage();
      const localeMap = {
        en: 'en-US',
        zh: 'zh-CN',
        ja: 'ja-JP'
      };
      return date.toLocaleString(localeMap[currentLocale] || 'en-US', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
    } catch {
      return dateString;
    }
  },
  
  // 格式化相对时间
  relativeTime: (dateString: string): string => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffMs = now.getTime() - date.getTime();
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMs / 3600000);
      const diffDays = Math.floor(diffMs / 86400000);
      
      if (diffMins < 1) return 'just now';
      if (diffMins < 60) return `${diffMins}m ago`;
      if (diffHours < 24) return `${diffHours}h ago`;
      if (diffDays < 7) return `${diffDays}d ago`;
      
      return format.date(dateString);
    } catch {
      return dateString;
    }
  }
};