"use client";

import { Button } from "@/components/ui/button";
import { Plus, Search, ChevronRight, Star } from "lucide-react";

interface EntryListHeaderProps {
  isSearching: boolean;
  selectedGroupName?: string;
  entryCount: number;
  groupUuid: string;
  onCreateClick: () => void;
}

export function EntryListHeader({
  isSearching,
  selectedGroupName,
  entryCount,
  groupUuid,
  onCreateClick,
}: EntryListHeaderProps) {
  return (
    <div className="flex items-center justify-between border-b px-4 py-2">
      <div className="flex items-center gap-3 flex-1 min-w-0">
        {isSearching && selectedGroupName === "Favorites" ? (
          <h2 className="text-sm font-semibold">Favorites</h2>
        ) : isSearching ? (
          <h2 className="text-sm font-semibold">Search Results</h2>
        ) : selectedGroupName && (
          <div className="flex items-center gap-1.5 text-sm font-semibold truncate">
            {selectedGroupName.split('/').map((segment, index, array) => (
              <div key={index} className="flex items-center gap-1.5">
                <span className="truncate">{segment}</span>
                {index < array.length - 1 && (
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                )}
              </div>
            ))}
          </div>
        )}
        <span className="text-sm text-muted-foreground whitespace-nowrap">
          ({entryCount})
        </span>
      </div>
      {isSearching && selectedGroupName === "Favorites" ? (
        <div className="h-7 w-7 flex items-center justify-center shrink-0">
          <Star className="h-4 w-4 text-muted-foreground" />
        </div>
      ) : isSearching ? (
        <div className="h-7 w-7 flex items-center justify-center shrink-0">
          <Search className="h-4 w-4 text-muted-foreground" />
        </div>
      ) : groupUuid && (
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 shrink-0"
          onClick={onCreateClick}
        >
          <Plus className="h-4 w-4" />
        </Button>
      )}
    </div>
  );
}
