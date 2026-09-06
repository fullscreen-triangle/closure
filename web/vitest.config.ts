import { defineConfig, mergeConfig } from 'vitest/config'
import base from './vite.config'

/**
 * Tests run in `node`, not jsdom.
 *
 * What is worth testing here is the client's reading of the wire and of the
 * theory — token normalisation, the error path, the speaker union, the act
 * table. None of that needs a DOM, and pulling in jsdom plus a testing
 * library to render components would buy snapshots of markup that changes
 * whenever the copy does.
 */
export default mergeConfig(
  base,
  defineConfig({
    test: {
      environment: 'node',
      include: ['src/**/*.test.ts'],
    },
  }),
)
