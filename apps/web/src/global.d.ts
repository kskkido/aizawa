type DeepNonNullable<T> =
  T extends Array<infer U>
    ? Array<DeepNonNullable<U>>
    : T extends object
      ? { [K in keyof T]-?: DeepNonNullable<NonNullable<T[K]>> }
      : NonNullable<T>;

type WithRequired<T, K extends keyof T> = Omit<T, K> & Required<Pick<T, K>>;

declare module "@design-system/build/js/tokens" {
  export const ColorSurfaceNormal: string;
}
