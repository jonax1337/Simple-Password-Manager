"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Edit, X, ShieldAlert, Trash2 } from "lucide-react";
import { RefreshCcw } from "@/components/animate-ui/icons/refresh-ccw";
import { checkBreachedPasswords } from "@/lib/tauri";
import type { BreachedEntry } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import { saveDismissedBreach, getDismissedBreaches } from "@/lib/storage";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";

interface BreachedPasswordsCardProps {
  refreshTrigger?: number;
  databasePath?: string;
  onEditEntry: (entryUuid: string) => void;
  isDirty?: boolean;
}

export function BreachedPasswordsCard({ refreshTrigger, databasePath, onEditEntry, isDirty }: BreachedPasswordsCardProps) {
  const [breachedEntries, setBreachedEntries] = useState<BreachedEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [dismissedUuids, setDismissedUuids] = useState<string[]>([]);
  const [selectedUuids, setSelectedUuids] = useState<Set<string>>(new Set());
  const [isInitialized, setIsInitialized] = useState(false);
  const [lastCheckTrigger, setLastCheckTrigger] = useState<number | undefined>(undefined);
  const { toast } = useToast();

  useEffect(() => {
    if (databasePath) {
      getDismissedBreaches(databasePath)
        .then(dismissed => {
          setDismissedUuids(dismissed);
        })
        .catch(error => {
          console.error("Failed to load dismissed breaches:", error);
          toast({
            title: "Warning",
            description: "Could not load dismissed breach warnings",
            variant: "destructive",
          });
        });
    }
  }, [databasePath]);

  // Only check breached passwords on initial load or when database has unsaved changes
  useEffect(() => {
    if (!databasePath) return;
    
    // Initial load
    if (!isInitialized) {
      loadBreachedPasswords();
      return;
    }
    
    // Only reload if there are unsaved changes (database was modified)
    // This prevents unnecessary recalculations on every refresh
    if (isDirty && lastCheckTrigger !== refreshTrigger) {
      setLastCheckTrigger(refreshTrigger);
      loadBreachedPasswords();
    }
  }, [refreshTrigger, databasePath, isDirty, isInitialized, lastCheckTrigger]);

  const loadBreachedPasswords = async () => {
    setIsLoading(true);
    try {
      const breached = await checkBreachedPasswords();
      setBreachedEntries(breached);
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to check for breached passwords"),
        variant: "destructive",
      });
    } finally {
      setIsLoading(false);
      setIsInitialized(true);
    }
  };

  const handleDismiss = async (entryUuid: string) => {
    if (!databasePath) {
      toast({
        title: "Error",
        description: "Database path is not available",
        variant: "destructive",
      });
      return;
    }
    
    try {
      await saveDismissedBreach(databasePath, entryUuid);
      setDismissedUuids(prev => [...prev, entryUuid]);
      toast({
        title: "Dismissed",
        description: "This breach warning will no longer be shown",
        variant: "info",
      });
    } catch (error: any) {
      console.error("Failed to dismiss breach:", error);
      toast({
        title: "Error",
        description: error?.message || "Failed to dismiss breach warning",
        variant: "destructive",
      });
    }
  };

  const handleBulkDismiss = async () => {
    if (!databasePath) {
      toast({
        title: "Error",
        description: "Database path is not available",
        variant: "destructive",
      });
      return;
    }
    
    if (selectedUuids.size === 0) {
      toast({
        title: "No Selection",
        description: "Please select entries to dismiss",
        variant: "destructive",
      });
      return;
    }
    
    try {
      const results = await Promise.allSettled(
        Array.from(selectedUuids).map(uuid => saveDismissedBreach(databasePath, uuid))
      );
      
      const successful = results.filter(r => r.status === 'fulfilled').length;
      const failed = results.filter(r => r.status === 'rejected').length;
      
      if (successful > 0) {
        setDismissedUuids(prev => [...prev, ...Array.from(selectedUuids)]);
        setSelectedUuids(new Set());
      }
      
      if (failed > 0) {
        toast({
          title: "Partial Success",
          description: `${successful} dismissed, ${failed} failed`,
          variant: "destructive",
        });
      } else {
        toast({
          title: "Dismissed",
          description: `${successful} breach warning${successful !== 1 ? 's' : ''} dismissed`,
          variant: "info",
        });
      }
    } catch (error: any) {
      console.error("Failed to bulk dismiss breaches:", error);
      toast({
        title: "Error",
        description: error?.message || "Failed to dismiss breach warnings",
        variant: "destructive",
      });
    }
  };

  const toggleSelectEntry = (uuid: string) => {
    const newSelected = new Set(selectedUuids);
    if (newSelected.has(uuid)) {
      newSelected.delete(uuid);
    } else {
      newSelected.add(uuid);
    }
    setSelectedUuids(newSelected);
  };

  const toggleSelectAll = () => {
    if (selectedUuids.size === filteredBreachedEntries.length && filteredBreachedEntries.length > 0) {
      setSelectedUuids(new Set());
    } else {
      setSelectedUuids(new Set(filteredBreachedEntries.map(e => e.uuid)));
    }
  };

  const clearSelection = () => {
    setSelectedUuids(new Set());
  };

  const filteredBreachedEntries = breachedEntries.filter(
    entry => !dismissedUuids.includes(entry.uuid)
  );

  // Show nothing if no breached entries and not loading (and initialized)
  if (isInitialized && filteredBreachedEntries.length === 0 && !isLoading) {
    return null;
  }
  
  // Show nothing while initial load is happening (before any data)
  if (!isInitialized && breachedEntries.length === 0 && isLoading) {
    return null;
  }

  return (
    <Card className="border-red-200 dark:border-red-900">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ShieldAlert className="h-5 w-5 text-red-500" />
            <CardTitle className="text-red-600 dark:text-red-400">Breached Passwords</CardTitle>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={loadBreachedPasswords}
            disabled={isLoading}
            className="h-8 w-8"
          >
            <RefreshCcw 
              size={16} 
              animate={isLoading ? "rotate" : undefined}
              animateOnHover={!isLoading}
            />
          </Button>
        </div>
        <CardDescription>
          These passwords have been found in data breaches and should be changed immediately
        </CardDescription>
      </CardHeader>
      <CardContent className="p-0">
        {isLoading ? (
          <div className="flex items-center justify-center py-8 text-sm text-muted-foreground">
            <RefreshCcw size={16} className="animate-spin mr-2" />
            Checking passwords against HIBP database...
          </div>
        ) : filteredBreachedEntries.length === 0 ? (
          <div className="flex items-center justify-center py-8 text-sm text-muted-foreground">
            No breached passwords found
          </div>
        ) : (
          <div className="flex flex-col">
            {/* Selection Toolbar */}
            {selectedUuids.size > 0 && (
              <div className="flex items-center justify-between border-b px-4 py-2 bg-red-50 dark:bg-red-950/20">
                <div className="flex items-center gap-3">
                  <span className="text-sm font-semibold text-red-700 dark:text-red-400">
                    {selectedUuids.size} {selectedUuids.size === 1 ? 'entry' : 'entries'} selected
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={clearSelection}
                    className="h-7 text-xs"
                  >
                    <X className="h-3 w-3 mr-1" />
                    Clear
                  </Button>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleBulkDismiss}
                    className="h-7 text-xs"
                  >
                    <Trash2 className="h-3 w-3 mr-1" />
                    Dismiss All
                  </Button>
                </div>
              </div>
            )}

            {/* Entries */}
            <ScrollArea className="max-h-[400px]">
              {/* Table Header */}
              <div className="flex items-center gap-2 border-b bg-muted/50 px-4 py-2 text-xs font-semibold text-muted-foreground">
                <div className="w-8 flex items-center justify-center">
                  <Checkbox
                    checked={selectedUuids.size === filteredBreachedEntries.length && filteredBreachedEntries.length > 0}
                    onCheckedChange={toggleSelectAll}
                    title="Select all"
                  />
                </div>
                <div className="w-8"></div>
                <div className="flex-1">Title</div>
                <div className="flex-1">Username</div>
                <div className="w-32">Breach Count</div>
                <div className="w-24">Actions</div>
              </div>

              {filteredBreachedEntries.map((entry, index) => (
                <div
                  key={entry.uuid}
                  className={`flex items-center gap-2 border-b px-4 py-2.5 hover:bg-red-50 dark:hover:bg-red-950/20 cursor-pointer select-none ${
                    selectedUuids.has(entry.uuid) ? "bg-red-50 dark:bg-red-950/20" : ""
                  } ${index === filteredBreachedEntries.length - 1 ? "rounded-b-lg border-b-0" : ""}`}
                  onDoubleClick={() => onEditEntry(entry.uuid)}
                >
                  {/* Checkbox */}
                  <div className="flex items-center w-8 justify-center shrink-0">
                    <Checkbox
                      checked={selectedUuids.has(entry.uuid)}
                      onCheckedChange={() => toggleSelectEntry(entry.uuid)}
                      onClick={(e) => e.stopPropagation()}
                    />
                  </div>
                  
                  {/* Icon */}
                  <div className="flex items-center w-8 shrink-0">
                    <AlertTriangle className="h-4 w-4 text-red-500" />
                  </div>
                  
                  {/* Title */}
                  <div className="flex-1 overflow-hidden min-w-0">
                    <p className="truncate text-sm font-medium">{entry.title}</p>
                  </div>
                  
                  {/* Username */}
                  <div className="flex-1 overflow-hidden min-w-0">
                    <p className="truncate text-sm text-muted-foreground">
                      {entry.username || "—"}
                    </p>
                  </div>
                  
                  {/* Breach Count */}
                  <div className="w-32 shrink-0">
                    <p className="text-sm text-red-600 dark:text-red-400">
                      {entry.breach_count.toLocaleString()} breach{entry.breach_count !== 1 ? 'es' : ''}
                    </p>
                  </div>
                  
                  {/* Actions */}
                  <div className="w-24 flex items-center gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={(e) => {
                        e.stopPropagation();
                        onEditEntry(entry.uuid);
                      }}
                      className="h-7 w-7"
                      title="Edit entry"
                    >
                      <Edit className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDismiss(entry.uuid);
                      }}
                      className="h-7 w-7"
                      title="Dismiss this warning"
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
              ))}
            </ScrollArea>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
