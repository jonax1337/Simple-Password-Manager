"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { Eye, EyeOff, Copy, Wand2, Check, Clock } from "lucide-react";
import { PasswordStrengthMeter } from "@/components/PasswordStrengthMeter";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useToast } from "@/components/ui/use-toast";
import { getDefaultExpiryDate, getExpiryDate } from "./utils/formatters";
import { TotpSection } from "./TotpSection";
import type { EntryData } from "@/lib/tauri";

interface GeneralTabProps {
  formData: EntryData;
  repeatPassword: string;
  showPassword: boolean;
  urlError: string | null;
  copiedUsername: boolean;
  copiedPassword: boolean;
  onFormChange: (field: keyof EntryData, value: string) => void;
  onRepeatPasswordChange: (value: string) => void;
  onShowPasswordToggle: () => void;
  onCopy: (text: string, label: string, field: 'username' | 'password') => void;
  onGeneratePassword: (strength: string) => void;
  setFormData: React.Dispatch<React.SetStateAction<EntryData>>;
  setHasChanges: React.Dispatch<React.SetStateAction<boolean>>;
}

export function GeneralTab({
  formData,
  repeatPassword,
  showPassword,
  urlError,
  copiedUsername,
  copiedPassword,
  onFormChange,
  onRepeatPasswordChange,
  onShowPasswordToggle,
  onCopy,
  onGeneratePassword,
  setFormData,
  setHasChanges,
}: GeneralTabProps) {
  const { toast } = useToast();

  const handleExpiresChange = (checked: boolean) => {
    const updates: Partial<EntryData> = { expires: checked };
    
    // Set default expiry time to 1 year from now if enabling and no time set
    if (checked && !formData.expiry_time) {
      updates.expiry_time = getDefaultExpiryDate();
    }
    
    setFormData(prev => ({ ...prev, ...updates }));
    setHasChanges(true);
  };

  const handleExpiryTimeChange = (value: string) => {
    setFormData(prev => ({ 
      ...prev, 
      expiry_time: value,
      expires: true // Auto-enable expires when setting a time
    }));
    setHasChanges(true);
  };

  const handleExpiryPresetChange = (preset: string) => {
    setFormData(prev => ({ 
      ...prev, 
      expiry_time: getExpiryDate(preset),
      expires: true
    }));
    setHasChanges(true);
  };

  return (
    <ScrollArea className="h-full">
      <div className="p-4 space-y-4">
        {/* Title Row */}
        <div className="grid grid-cols-[80px_1fr] items-center gap-2">
          <Label htmlFor="title">Title:</Label>
          <Input
            id="title"
            value={formData.title}
            onChange={(e) => onFormChange("title", e.target.value)}
            placeholder="Entry title"
          />
        </div>

        {/* Username Row */}
        <div className="grid grid-cols-[80px_1fr_auto] items-center gap-2">
          <Label htmlFor="username">User name:</Label>
          <Input
            id="username"
            value={formData.username}
            onChange={(e) => onFormChange("username", e.target.value)}
          />
          <Button
            variant="outline"
            size="icon"
            onClick={() => onCopy(formData.username, "Username", "username")}
            disabled={!formData.username}
            title="Copy username"
            className="h-10 w-10"
          >
            {copiedUsername ? (
              <Check className="h-4 w-4 text-green-600" />
            ) : (
              <Copy className="h-4 w-4" />
            )}
          </Button>
        </div>

        {/* Password Row */}
        <div className="grid grid-cols-[80px_1fr_auto] items-center gap-2">
          <Label htmlFor="password">Password:</Label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              value={formData.password}
              onChange={(e) => onFormChange("password", e.target.value)}
              className="pr-10"
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute right-0 top-0 h-full"
              onClick={onShowPasswordToggle}
            >
              {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </Button>
          </div>
          <Button
            variant="outline"
            size="icon"
            onClick={() => onCopy(formData.password, "Password", "password")}
            disabled={!formData.password}
            title="Copy password"
            className="h-10 w-10"
          >
            {copiedPassword ? (
              <Check className="h-4 w-4 text-green-600" />
            ) : (
              <Copy className="h-4 w-4" />
            )}
          </Button>
        </div>

        {/* Repeat Password Row */}
        <div className="grid grid-cols-[80px_1fr_auto] items-center gap-2">
          <Label htmlFor="repeat">Repeat:</Label>
          <Input
            id="repeat"
            type="password"
            value={repeatPassword}
            onChange={(e) => onRepeatPasswordChange(e.target.value)}
            onPaste={(e) => {
              e.preventDefault();
              toast({
                title: "Paste disabled",
                description: "Please type the password to confirm",
                variant: "destructive",
              });
            }}
            className={repeatPassword && formData.password && formData.password !== repeatPassword ? "border-red-500" : ""}
            disabled={showPassword}
          />
          <Select value="" onValueChange={onGeneratePassword}>
            <SelectTrigger className="h-10 w-10 px-0 justify-center [&>svg[aria-hidden]]:hidden" title="Generate password">
              <Wand2 className="h-4 w-4" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="weak">Weak (8)</SelectItem>
              <SelectItem value="medium">Medium (12)</SelectItem>
              <SelectItem value="strong">Strong (16)</SelectItem>
              <SelectItem value="very-strong">Very Strong (20)</SelectItem>
              <SelectItem value="maximum">Maximum (32)</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Quality/Strength Row */}
        <div className="grid grid-cols-[80px_1fr] items-center gap-2">
          <Label>Quality:</Label>
          <PasswordStrengthMeter password={formData.password} />
        </div>

        {/* URL Row */}
        <div className="grid grid-cols-[80px_1fr] items-center gap-2">
          <Label htmlFor="url">URL:</Label>
          <div className="space-y-1">
            <Input
              id="url"
              type="url"
              value={formData.url}
              onChange={(e) => onFormChange("url", e.target.value)}
              className={urlError ? "border-red-500" : ""}
              placeholder="https://example.com"
            />
            {urlError && <p className="text-xs text-red-500">{urlError}</p>}
          </div>
        </div>

        {/* Two-factor authentication (TOTP) */}
        <TotpSection
          formData={formData}
          setFormData={setFormData}
          setHasChanges={setHasChanges}
        />

        {/* Notes */}
        <div className="grid grid-cols-[80px_1fr] items-start gap-2">
          <Label htmlFor="notes" className="pt-2">Notes:</Label>
          <textarea
            id="notes"
            value={formData.notes}
            onChange={(e) => onFormChange("notes", e.target.value)}
            className="flex min-h-[100px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            placeholder="Additional notes..."
          />
        </div>

        {/* Expires */}
        <div className="grid grid-cols-[80px_1fr] items-center gap-2">
          <div className="flex items-center gap-2">
            <Checkbox 
              id="expires"
              checked={formData.expires}
              onCheckedChange={(checked) => handleExpiresChange(checked === true)}
            />
            <Label htmlFor="expires">Expires:</Label>
          </div>
          <div className="flex gap-2 w-full">
            <Input
              type="datetime-local"
              value={formData.expiry_time?.slice(0, 16) || ''}
              onChange={(e) => handleExpiryTimeChange(e.target.value)}
              disabled={!formData.expires}
              className="flex-1"
            />
            <Select
              value=""
              onValueChange={handleExpiryPresetChange}
              disabled={!formData.expires}
            >
              <SelectTrigger className="h-10 w-10 px-0 justify-center [&>svg[aria-hidden]]:hidden" title="Set expiry preset">
                <Clock className="h-4 w-4" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="1day">1 Day</SelectItem>
                <SelectItem value="1week">1 Week</SelectItem>
                <SelectItem value="2weeks">2 Weeks</SelectItem>
                <SelectItem value="1month">1 Month</SelectItem>
                <SelectItem value="3months">3 Months</SelectItem>
                <SelectItem value="6months">6 Months</SelectItem>
                <SelectItem value="1year">1 Year</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}
