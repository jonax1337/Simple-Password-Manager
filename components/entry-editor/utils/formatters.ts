export const formatTimestamp = (timestamp?: string): string => {
  if (!timestamp) return "—";
  try {
    const date = new Date(timestamp);
    return new Intl.DateTimeFormat('de-DE', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false
    }).format(date);
  } catch {
    return "—";
  }
};

export const getExpiryDate = (preset: string): string => {
  const now = new Date();
  let targetDate = new Date();
  
  switch(preset) {
    case '1day':
      targetDate.setDate(now.getDate() + 1);
      break;
    case '1week':
      targetDate.setDate(now.getDate() + 7);
      break;
    case '2weeks':
      targetDate.setDate(now.getDate() + 14);
      break;
    case '1month':
      targetDate.setMonth(now.getMonth() + 1);
      break;
    case '3months':
      targetDate.setMonth(now.getMonth() + 3);
      break;
    case '6months':
      targetDate.setMonth(now.getMonth() + 6);
      break;
    case '1year':
      targetDate.setFullYear(now.getFullYear() + 1);
      break;
  }
  
  const year = targetDate.getFullYear();
  const month = String(targetDate.getMonth() + 1).padStart(2, '0');
  const day = String(targetDate.getDate()).padStart(2, '0');
  const hours = String(targetDate.getHours()).padStart(2, '0');
  const minutes = String(targetDate.getMinutes()).padStart(2, '0');
  
  return `${year}-${month}-${day}T${hours}:${minutes}`;
};

export const getDefaultExpiryDate = (): string => {
  const oneYearFromNow = new Date();
  oneYearFromNow.setFullYear(oneYearFromNow.getFullYear() + 1);
  // datetime-local input works in local time, backend stores as naive datetime
  const year = oneYearFromNow.getFullYear();
  const month = String(oneYearFromNow.getMonth() + 1).padStart(2, '0');
  const day = String(oneYearFromNow.getDate()).padStart(2, '0');
  const hours = String(oneYearFromNow.getHours()).padStart(2, '0');
  const minutes = String(oneYearFromNow.getMinutes()).padStart(2, '0');
  return `${year}-${month}-${day}T${hours}:${minutes}`;
};
