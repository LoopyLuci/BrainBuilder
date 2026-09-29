export function logInfo(title: string, data: unknown) {
  console.info(`[${title}]`, data);
}

export function logError(title: string, error: unknown) {
  console.error(`[${title}]`, error);
}
