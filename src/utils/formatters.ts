/**
 * Utility functions for formatting storage bytes, numbers and dates
 */

/**
 * Formats bytes into human-readable string (e.g., "1.42 GB", "350 MB")
 * Uses standard 1024 binary units (KiB, MiB, GiB, TiB) formatted as KB, MB, GB, TB.
 */
export function formatBytes(bytes: number, decimals: number = 2): string {
  if (bytes === 0) return '0 B';
  if (bytes < 0) return '-' + formatBytes(-bytes, decimals);

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const safeIndex = Math.min(i, sizes.length - 1);
  const value = bytes / Math.pow(k, safeIndex);

  return `${parseFloat(value.toFixed(dm))} ${sizes[safeIndex]}`;
}

/**
 * Formats a percentage number with 1 decimal place (e.g., "78.4%")
 */
export function formatPercent(percent: number): string {
  return `${percent.toFixed(1)}%`;
}

/**
 * Formats large counts with commas (e.g. 1,425,890)
 */
export function formatNumber(count: number): string {
  return new Intl.NumberFormat().format(count);
}

