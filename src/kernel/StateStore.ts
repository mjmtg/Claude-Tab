/**
 * StateStore - Generic Reactive State Container
 *
 * A simple state management pattern to replace module-level state variables.
 * Provides a reactive container that notifies subscribers on state changes.
 */

export interface StateStore<T> {
  /** Get the current state */
  getState(): T;

  /** Update state with partial values */
  setState(partial: Partial<T>): void;

  /** Replace the entire state */
  replaceState(state: T): void;

  /** Subscribe to state changes, returns unsubscribe function */
  subscribe(listener: (state: T) => void): () => void;

  /** Get a specific property from state */
  get<K extends keyof T>(key: K): T[K];

  /** Set a specific property in state */
  set<K extends keyof T>(key: K, value: T[K]): void;
}

type StateListener<T> = (state: T) => void;

/**
 * Create a new state store with the given initial state.
 *
 * @example
 * ```ts
 * interface CounterState {
 *   count: number;
 *   label: string;
 * }
 *
 * const store = createStateStore<CounterState>({
 *   count: 0,
 *   label: "Counter",
 * });
 *
 * // Subscribe to changes
 * const unsubscribe = store.subscribe((state) => {
 *   console.log("State changed:", state);
 * });
 *
 * // Update state
 * store.setState({ count: 1 });
 * store.set("label", "My Counter");
 *
 * // Get state
 * const count = store.get("count");
 * const fullState = store.getState();
 *
 * // Cleanup
 * unsubscribe();
 * ```
 */
export function createStateStore<T extends object>(initialState: T): StateStore<T> {
  let state: T = { ...initialState };
  const listeners: Set<StateListener<T>> = new Set();

  const notify = () => {
    const currentState = state;
    listeners.forEach((listener) => {
      try {
        listener(currentState);
      } catch (err) {
        console.error("[StateStore] Listener error:", err);
      }
    });
  };

  return {
    getState() {
      return state;
    },

    setState(partial: Partial<T>) {
      state = { ...state, ...partial };
      notify();
    },

    replaceState(newState: T) {
      state = { ...newState };
      notify();
    },

    subscribe(listener: StateListener<T>) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },

    get<K extends keyof T>(key: K): T[K] {
      return state[key];
    },

    set<K extends keyof T>(key: K, value: T[K]) {
      state = { ...state, [key]: value };
      notify();
    },
  };
}

/**
 * React hook to use a StateStore.
 * Re-renders the component when state changes.
 */
import { useState, useEffect, useCallback } from "react";

export function useStateStore<T extends object>(store: StateStore<T>): [T, StateStore<T>["setState"]] {
  const [state, setState] = useState<T>(store.getState());

  useEffect(() => {
    const unsubscribe = store.subscribe((newState) => {
      setState(newState);
    });
    // Sync state in case it changed between render and effect
    setState(store.getState());
    return unsubscribe;
  }, [store]);

  const updateState = useCallback(
    (partial: Partial<T>) => {
      store.setState(partial);
    },
    [store]
  );

  return [state, updateState];
}

/**
 * React hook to select a specific value from a StateStore.
 * Only re-renders when the selected value changes.
 */
export function useStoreSelector<T extends object, R>(
  store: StateStore<T>,
  selector: (state: T) => R
): R {
  const [value, setValue] = useState<R>(() => selector(store.getState()));

  useEffect(() => {
    const unsubscribe = store.subscribe((state) => {
      const newValue = selector(state);
      setValue((prev) => {
        // Only update if value actually changed
        if (prev !== newValue) {
          return newValue;
        }
        return prev;
      });
    });
    // Sync in case it changed
    setValue(selector(store.getState()));
    return unsubscribe;
  }, [store, selector]);

  return value;
}
