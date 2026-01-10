"use client";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ExternalLink, Github } from "lucide-react";

interface AboutDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AboutDialog({ open, onOpenChange }: AboutDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center gap-3 mb-2">
            <img 
              src="/app-icon.png" 
              alt="App Icon" 
              className="h-12 w-12"
            />
            <div>
              <DialogTitle className="text-2xl">Simple Password Manager</DialogTitle>
              <DialogDescription className="mt-1">
                Version 1.0.0
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <p className="text-sm text-muted-foreground">
            A secure, modern password manager built with KeePass database format (.kdbx). 
            Keep your passwords safe with strong encryption and a clean, intuitive interface.
          </p>

          <div className="space-y-2">
            <h4 className="text-sm font-semibold">Features</h4>
            <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
              <li>KeePass 2.x database format support</li>
              <li>Strong encryption (AES-256, ChaCha20)</li>
              <li>Password generator with strength meter</li>
              <li>Breach checking with Have I Been Pwned</li>
              <li>Auto-lock and quick unlock</li>
              <li>Cross-platform support</li>
            </ul>
          </div>

          <div className="flex gap-2 pt-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => window.open('https://github.com/jonax1337/Password-Manager', '_blank')}
              className="flex-1"
            >
              <Github className="h-4 w-4 mr-2" />
              GitHub Repository
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => window.open('https://github.com/jonax1337/Password-Manager/issues', '_blank')}
              className="flex-1"
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              Report Issue
            </Button>
          </div>

          <div className="pt-2 border-t">
            <p className="text-xs text-muted-foreground text-center">
              Built with Tauri, React, and Next.js
            </p>
            <p className="text-xs text-muted-foreground text-center mt-1">
              © 2026 Simple Password Manager. Open Source Software.
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
