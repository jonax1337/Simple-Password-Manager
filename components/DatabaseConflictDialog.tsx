"use client";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AlertCircle, ArrowDownUp, FileDown, X } from "lucide-react";

interface DatabaseConflictDialogProps {
  open: boolean;
  databasePath: string;
  onSynchronize: () => void;
  onOverwrite: () => void;
  onCancel: () => void;
}

export function DatabaseConflictDialog({
  open,
  databasePath,
  onSynchronize,
  onOverwrite,
  onCancel,
}: DatabaseConflictDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-blue-500" />
            <DialogTitle>Overwrite the existing file?</DialogTitle>
          </div>
          <DialogDescription asChild>
            <div className="pt-2 space-y-2">
              <div className="font-mono text-sm bg-muted p-2 rounded">
                {databasePath}
              </div>
              <div>
                The file on disk/server has changed since it was loaded. Probably someone else
                has edited and saved the database.
              </div>
            </div>
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-2 py-2">
          <button
            onClick={onSynchronize}
            className="w-full flex items-start gap-3 p-3 rounded-md border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
          >
            <ArrowDownUp className="h-5 w-5 text-blue-500 shrink-0 mt-0.5" />
            <div className="flex-1 space-y-1">
              <div className="font-medium text-blue-600">Synchronize</div>
              <div className="text-sm text-muted-foreground">
                Load the file on disk/server and merge it with the current database in
                memory.
              </div>
            </div>
          </button>
          <button
            onClick={onOverwrite}
            className="w-full flex items-start gap-3 p-3 rounded-md border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
          >
            <FileDown className="h-5 w-5 text-blue-500 shrink-0 mt-0.5" />
            <div className="flex-1 space-y-1">
              <div className="font-medium text-blue-600">Overwrite</div>
              <div className="text-sm text-muted-foreground">
                Save the current database to the file. Changes made by the other user
                will be lost.
              </div>
            </div>
          </button>
          <button
            onClick={onCancel}
            className="w-full flex items-start gap-3 p-3 rounded-md border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
          >
            <X className="h-5 w-5 text-blue-500 shrink-0 mt-0.5" />
            <div className="flex-1 space-y-1">
              <div className="font-medium text-blue-600">Cancel</div>
              <div className="text-sm text-muted-foreground">
                Abort the current operation.
              </div>
            </div>
          </button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
