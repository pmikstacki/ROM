// Closed diagnostic projections. Never retain request values or unrecognized field names.
export function requireCaptureAttempt(value) {
  if (!/^attempt(?:[1-9]|[12][0-9]|3[0-2])$/.test(value)) throw Error('invalid capture attempt name');
  return value;
}

export function requestFieldNames(contentType, body) {
  if (typeof body !== 'string' || body.length > 65536) return [];
  try {
    const value = contentType?.startsWith('application/json') ? JSON.parse(body)
      : contentType?.startsWith('application/x-www-form-urlencoded') ? Object.fromEntries(new URLSearchParams(body))
        : null;
    if (!value || typeof value !== 'object' || Array.isArray(value)) return [];
    return Object.keys(value).filter(name => ['component', 'password', 'username', 'uid_field', 'uidField', 'continue'].includes(name)).sort();
  } catch { return []; }
}
