import react from '@vitejs/plugin-react';
import { configDefaults, defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],
  // `.claude/` holds agent worktrees: full copies of the repo whose tests would run twice.
  test: { environment: 'jsdom', exclude: [...configDefaults.exclude, '.claude/**'] },
});
