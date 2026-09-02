import js from '@eslint/js'
import ts from 'typescript-eslint'
import hooks from 'eslint-plugin-react-hooks'

export default ts.config(
  { ignores: ['dist', 'node_modules', 'eslint.config.js', 'postcss.config.js', 'tailwind.config.js'] },

  // Plain JS rules everywhere.
  js.configs.recommended,

  // Type-aware rules apply only to the TypeScript sources, which are the
  // files the tsconfig project actually covers. Applying them to the config
  // files themselves fails, since those are outside the project.
  {
    files: ['**/*.{ts,tsx}'],
    extends: [...ts.configs.recommendedTypeChecked],
    languageOptions: {
      parserOptions: {
        project: './tsconfig.json',
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: { 'react-hooks': hooks },
    rules: {
      ...hooks.configs.recommended.rules,
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  },
)
