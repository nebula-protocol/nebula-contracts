import { combineReducers, createStore } from 'redux';

export const rootReducer = combineReducers({});

export default function configureStore(preloadedState: any) {
  const store = createStore(
    rootReducer,
    preloadedState,
    (window as any).__REDUX_DEVTOOLS_EXTENSION__ &&
      (window as any).__REDUX_DEVTOOLS_EXTENSION__()
  );
  return store;
}

export const store = configureStore({});
