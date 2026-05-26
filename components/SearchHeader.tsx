"use client";

import { Search, Globe, Folder, ChevronDown, ChevronUp } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useRef, useEffect } from "react";

interface SearchHeaderProps {
  searchQuery?: string;
  onSearchChange?: (query: string) => void;
  searchScope?: "global" | "folder";
  onSearchScopeChange?: (scope: "global" | "folder") => void;
  showSearchScopeDropdown?: boolean;
  isSearchScopeDisabled?: boolean;
  isVisible?: boolean;
  onToggle?: () => void;
}

export function SearchHeader({
  searchQuery = "",
  onSearchChange,
  searchScope = "global",
  onSearchScopeChange,
  showSearchScopeDropdown = false,
  isSearchScopeDisabled = false,
  isVisible = true,
  onToggle,
}: SearchHeaderProps) {
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Focus search bar when visible
  useEffect(() => {
    if (isVisible) {
      setTimeout(() => {
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      }, 100);
    }
  }, [isVisible]);

  return (
    <div 
      className={`bg-background/95 backdrop-blur-xs border-b px-6 transition-all duration-300 ease-in-out overflow-hidden ${
        isVisible ? 'py-3 max-h-24' : 'py-0 max-h-0 border-b-0'
      }`}
    >
      <div className="max-w-4xl mx-auto relative">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            ref={searchInputRef}
            placeholder="Search entries..."
            className={`h-10 pl-10 text-sm ${showSearchScopeDropdown ? 'pr-32' : 'pr-4'} border-2 focus-visible:ring-2 focus-visible:ring-offset-0`}
            value={searchQuery}
            onChange={(e) => onSearchChange?.(e.target.value)}
          />
          {showSearchScopeDropdown && (
            <div className="absolute right-2 top-1/2 -translate-y-1/2">
              <DropdownMenu>
                <DropdownMenuTrigger asChild disabled={isSearchScopeDisabled}>
                  <Button
                    variant="outline"
                    size="sm"
                    className={`h-7 gap-2 px-3 text-xs font-medium ${isSearchScopeDisabled ? 'opacity-50 cursor-not-allowed' : ''}`}
                    disabled={isSearchScopeDisabled}
                  >
                    {searchScope === "global" ? (
                      <>
                        <Globe className="h-3.5 w-3.5" />
                        <span>Global</span>
                      </>
                    ) : (
                      <>
                        <Folder className="h-3.5 w-3.5" />
                        <span>Folder</span>
                      </>
                    )}
                    <ChevronDown className="h-3 w-3 opacity-50" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-40">
                  <DropdownMenuItem
                    onClick={() => onSearchScopeChange?.("global")}
                    className="gap-2 cursor-pointer"
                  >
                    <Globe className="h-4 w-4" />
                    <span>Global</span>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => onSearchScopeChange?.("folder")}
                    className="gap-2 cursor-pointer"
                  >
                    <Folder className="h-4 w-4" />
                    <span>Folder</span>
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
