"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { getDashboardStats } from "@/lib/tauri";
import type { DashboardStats } from "@/lib/tauri";
import { 
  Shield, 
  Key, 
  AlertTriangle, 
  Clock, 
  Star, 
  FolderTree,
  TrendingUp,
  Copy
} from "lucide-react";
import { useToast } from "@/components/ui/use-toast";
import { BreachedPasswordsCard } from "@/components/BreachedPasswordsCard";
import { getEntry } from "@/lib/tauri";
import { getHibpEnabled } from "@/lib/storage";
import { openEntryWindow } from "@/lib/window";

interface DashboardProps {
  refreshTrigger?: number;
  databasePath?: string;
  isDirty?: boolean;
}

export function Dashboard({ refreshTrigger, databasePath, isDirty }: DashboardProps) {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [hibpEnabled, setHibpEnabled] = useState<boolean>(false);
  const { toast } = useToast();

  useEffect(() => {
    setHibpEnabled(getHibpEnabled());
  }, []);

  const handleEditEntry = async (entryUuid: string) => {
    try {
      const entry = await getEntry(entryUuid);
      await openEntryWindow(entry, entry.group_uuid);
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to open entry"),
        variant: "destructive",
      });
    }
  };

  useEffect(() => {
    loadStats();
  }, [refreshTrigger]);

  const loadStats = async () => {
    try {
      const data = await getDashboardStats();
      setStats(data);
    } catch (error: any) {
      toast({
        title: "Error",
        description: error?.message || "Failed to load dashboard statistics",
        variant: "destructive",
      });
    }
  };

  const getStrengthLevel = (bits: number): { label: string; color: string } => {
    if (bits < 40) return { label: "Weak", color: "text-red-500" };
    if (bits < 64) return { label: "Fair", color: "text-orange-500" };
    if (bits < 80) return { label: "Good", color: "text-yellow-500" };
    if (bits < 112) return { label: "Strong", color: "text-blue-500" };
    return { label: "Excellent", color: "text-green-500" };
  };

  const strengthInfo = stats ? getStrengthLevel(stats.average_password_strength) : { label: "...", color: "text-muted-foreground" };
  const healthScore = stats ? Math.max(
    0,
    100 - 
    (stats.weak_passwords / Math.max(stats.total_entries, 1)) * 30 -
    (stats.reused_passwords / Math.max(stats.total_entries, 1)) * 25 -
    (stats.old_passwords / Math.max(stats.total_entries, 1)) * 20 -
    (stats.expired_entries / Math.max(stats.total_entries, 1)) * 25
  ) : 0;

  const fadeInUp = {
    initial: { opacity: 0, y: 20 },
    animate: { opacity: 1, y: 0 },
    transition: { duration: 0.4, ease: "easeOut" }
  };

  const staggerContainer = {
    animate: {
      transition: {
        staggerChildren: 0.1
      }
    }
  };

  return (
    <div className="h-full overflow-auto">
      <div className="space-y-4 p-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, ease: "easeOut" }}
        >
          <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
          <p className="text-muted-foreground mt-1">
            Overview of your password database health and statistics
          </p>
        </motion.div>

        {hibpEnabled && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, ease: "easeOut", delay: 0.1 }}
          >
            <BreachedPasswordsCard
              refreshTrigger={refreshTrigger}
              databasePath={databasePath}
              onEditEntry={handleEditEntry}
              isDirty={isDirty}
            />
          </motion.div>
        )}

        <motion.div 
          className="grid gap-4 md:grid-cols-2 lg:grid-cols-4"
          variants={staggerContainer}
          initial="initial"
          animate="animate"
        >
          <motion.div variants={fadeInUp}>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Total Entries</CardTitle>
                <Key className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats?.total_entries ?? "..."}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  Across {stats?.total_groups ?? "..."} groups
                </p>
              </CardContent>
            </Card>
          </motion.div>

          <motion.div variants={fadeInUp}>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Health Score</CardTitle>
                <Shield className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats ? Math.round(healthScore) : "..."}%</div>
                <p className="text-xs text-muted-foreground mt-1">
                  Database security rating
                </p>
              </CardContent>
            </Card>
          </motion.div>

          <motion.div variants={fadeInUp}>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Avg. Strength</CardTitle>
                <TrendingUp className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className={`text-2xl font-bold ${strengthInfo.color}`}>
                  {strengthInfo.label}
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {stats ? Math.round(stats.average_password_strength) : "..."} bits entropy
                </p>
              </CardContent>
            </Card>
          </motion.div>

          <motion.div variants={fadeInUp}>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Favorites</CardTitle>
                <Star className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats?.favorite_entries ?? "..."}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  Marked as favorite
                </p>
              </CardContent>
            </Card>
          </motion.div>
        </motion.div>

        <motion.div 
          className="grid gap-4 md:grid-cols-2"
          variants={staggerContainer}
          initial="initial"
          animate="animate"
        >
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, ease: "easeOut", delay: 0.5 }}
          >
            <Card>
              <CardHeader>
                <CardTitle>Security Issues</CardTitle>
                <CardDescription>
                  Passwords that need attention
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="h-4 w-4 text-red-500" />
                    <span className="text-sm font-medium">Weak Passwords</span>
                  </div>
                  <span className={`text-lg font-bold ${stats && stats.weak_passwords > 0 ? 'text-red-500' : 'text-green-500'}`}>
                    {stats?.weak_passwords ?? "..."}
                  </span>
                </div>

                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Copy className="h-4 w-4 text-orange-500" />
                    <span className="text-sm font-medium">Reused Passwords</span>
                  </div>
                  <span className={`text-lg font-bold ${stats && stats.reused_passwords > 0 ? 'text-orange-500' : 'text-green-500'}`}>
                    {stats?.reused_passwords ?? "..."}
                  </span>
                </div>

                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Clock className="h-4 w-4 text-yellow-500" />
                    <span className="text-sm font-medium">Old Passwords</span>
                  </div>
                  <span className={`text-lg font-bold ${stats && stats.old_passwords > 0 ? 'text-yellow-500' : 'text-green-500'}`}>
                    {stats?.old_passwords ?? "..."}
                  </span>
                </div>

                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="h-4 w-4 text-red-500" />
                    <span className="text-sm font-medium">Expired Entries</span>
                  </div>
                  <span className={`text-lg font-bold ${stats && stats.expired_entries > 0 ? 'text-red-500' : 'text-green-500'}`}>
                    {stats?.expired_entries ?? "..."}
                  </span>
                </div>
              </CardContent>
            </Card>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, ease: "easeOut", delay: 0.6 }}
          >
            <Card>
              <CardHeader>
                <CardTitle>Recommendations</CardTitle>
                <CardDescription>
                  Actions to improve your security
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {!stats ? (
                  <div className="text-sm text-muted-foreground">Loading recommendations...</div>
                ) : stats.weak_passwords === 0 && 
                   stats.reused_passwords === 0 && 
                   stats.old_passwords === 0 && 
                   stats.expired_entries === 0 ? (
                  <div className="flex items-start gap-2 text-sm">
                    <Shield className="h-4 w-4 text-green-500 mt-0.5 shrink-0" />
                    <div>
                      <p className="font-medium text-green-600">All good!</p>
                      <p className="text-muted-foreground text-xs">
                        No security issues detected
                      </p>
                    </div>
                  </div>
                ) : (
                  <>
                {stats.weak_passwords > 0 && (
                  <div className="flex items-start gap-2 text-sm">
                    <AlertTriangle className="h-4 w-4 text-red-500 mt-0.5 shrink-0" />
                    <div>
                      <p className="font-medium">Update weak passwords</p>
                      <p className="text-muted-foreground text-xs">
                        {stats.weak_passwords} password{stats.weak_passwords !== 1 ? 's' : ''} with less than 40 bits of entropy
                      </p>
                    </div>
                  </div>
                )}

                {stats.reused_passwords > 0 && (
                  <div className="flex items-start gap-2 text-sm">
                    <Copy className="h-4 w-4 text-orange-500 mt-0.5 shrink-0" />
                    <div>
                      <p className="font-medium">Replace reused passwords</p>
                      <p className="text-muted-foreground text-xs">
                        {stats.reused_passwords} password{stats.reused_passwords !== 1 ? 's are' : ' is'} used multiple times
                      </p>
                    </div>
                  </div>
                )}

                {stats.old_passwords > 0 && (
                  <div className="flex items-start gap-2 text-sm">
                    <Clock className="h-4 w-4 text-yellow-500 mt-0.5 shrink-0" />
                    <div>
                      <p className="font-medium">Refresh old passwords</p>
                      <p className="text-muted-foreground text-xs">
                        {stats.old_passwords} password{stats.old_passwords !== 1 ? 's' : ''} older than 90 days
                      </p>
                    </div>
                  </div>
                )}

                {stats.expired_entries > 0 && (
                  <div className="flex items-start gap-2 text-sm">
                    <AlertTriangle className="h-4 w-4 text-red-500 mt-0.5 shrink-0" />
                    <div>
                      <p className="font-medium">Review expired entries</p>
                      <p className="text-muted-foreground text-xs">
                        {stats.expired_entries} entr{stats.expired_entries !== 1 ? 'ies have' : 'y has'} passed expiration date
                      </p>
                    </div>
                  </div>
                )}
                  </>
                )}
              </CardContent>
            </Card>
          </motion.div>
        </motion.div>
      </div>
    </div>
  );
}
