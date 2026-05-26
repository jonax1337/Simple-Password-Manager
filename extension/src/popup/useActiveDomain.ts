import { useEffect, useState } from "react";

export function useActiveDomain(): {
  domain: string | null;
  tabId: number | null;
  loading: boolean;
} {
  const [domain, setDomain] = useState<string | null>(null);
  const [tabId, setTabId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const tab = tabs[0];
      if (!tab) {
        setLoading(false);
        return;
      }
      setTabId(tab.id ?? null);
      try {
        if (tab.url) {
          const u = new URL(tab.url);
          if (u.protocol === "http:" || u.protocol === "https:") {
            setDomain(u.hostname.replace(/^www\./, ""));
          }
        }
      } catch {
        // ignore — tab.url might be chrome:// or similar
      }
      setLoading(false);
    });
  }, []);

  return { domain, tabId, loading };
}
