export type Store<K = string> = {
  get: (key: K) => string | null;
  set: (key: K, value: string) => void;
};

export const session = (prefix: string): Store => ({
  get: (key) => sessionStorage.getItem([prefix, key].join("-")),
  set: (key, value) => sessionStorage.setItem([prefix, key].join("-"), value),
});

export const memory = (() => {
  const data = new Map<string, string>();
  return (prefix: string): Store => ({
    get: (key) => data.get([prefix, key].join("-")) ?? null,
    set: (key, value) => data.set([prefix, key].join("-"), value),
  });
})();

export const slider = (id: string): Store<"slug"> =>
  memory(["slider", id].join("-"));
