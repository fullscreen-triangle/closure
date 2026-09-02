/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Base URL for the closure API. Empty in dev, where Vite proxies /v1. */
  readonly VITE_CLOSURE_API?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
