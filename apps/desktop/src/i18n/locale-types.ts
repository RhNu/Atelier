export type LocaleShape<T> = {
  readonly [K in keyof T]: T[K] extends object ? LocaleShape<T[K]> : string;
};
