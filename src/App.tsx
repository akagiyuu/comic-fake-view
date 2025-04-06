'use client';

import { useEffect, useMemo, useState } from 'react';
import './index.css';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import {
    FolderOpen,
    Play,
    CheckCircle2,
    Clock,
    Loader2,
    Moon,
    Sun,
    Monitor,
    EyeOff,
    Square,
    XCircle,
    Save,
} from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
    Form,
    FormControl,
    FormDescription,
    FormField,
    FormItem,
    FormLabel,
    FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { listen, once } from '@tauri-apps/api/event';
import convert from 'humanize-duration';
import { toast } from 'sonner';
import { useTheme } from '@/components/theme-provider';
import { Switch } from '@/components/ui/switch';

const formSchema = z.object({
    chrome_path: z.string().optional(),
    user_data_dir: z.string().optional(),
    wait_for_navigation: z.number(),
    max_retries: z.number().min(0).max(10),
    tab_count: z.number().min(1),
    headless: z.boolean(),
});

type FormValues = z.infer<typeof formSchema>;

const defaultValues: FormValues = await invoke('get_config');

function App() {
    const [totalJobs, setTotalJobs] = useState(10);
    const [completedJobs, setCompletedJobs] = useState(0);
    const [status, setStatus] = useState<
        'idle' | 'running' | 'completed' | 'stopped'
    >('idle');
    const { theme, setTheme } = useTheme();
    const [elapsed, setElapsed] = useState(0);
    const [isStoppable, setIsStoppable] = useState(false);

    useEffect(() => {
        listen('error', (error) => toast.error(String(error.payload)));
    }, []);

    const form = useForm<FormValues>({
        resolver: zodResolver(formSchema),
        defaultValues,
    });

    const saveConfiguration = async () => {
        try {
            await invoke('set_config', {
                config: form.getValues(),
            });
            toast.success('Configuration saved successfully');
        } catch (error) {
            toast.error('Failed to save configuration');
        }
    };

    const runAutomation = async () => {
        try {
            // Save configuration before running
            await invoke('set_config', {
                config: form.getValues(),
            });

            // Start the automation
            invoke('run');
            const start = performance.now();

            setStatus('running');
            setIsStoppable(true);

            once('total_jobs', async (event) => {
                setTotalJobs(Number(event.payload));
                setCompletedJobs(0);

                const unlisten = await listen('complete', () =>
                    setCompletedJobs((prev) => prev + 1),
                );

                await once('completed', async () => {
                    unlisten();
                    setElapsed(performance.now() - start);
                    setStatus('completed');
                    setIsStoppable(false);
                });
            });
        } catch (error) {
            toast.error('Failed to start automation');
            setStatus('idle');
            setIsStoppable(false);
        }
    };

    const stopAutomation = async () => {
        try {
            await invoke('stop');
            toast.info('Automation stopped');
            setStatus('stopped');
            setIsStoppable(false);
            setElapsed(performance.now() - elapsed);
        } catch (error) {
            toast.error('Failed to stop automation');
        }
    };

    // This would be implemented with actual file system access in a desktop app
    async function handleFilePicker(
        field: any,
        type: string,
        isDirectory = false,
    ) {
        const path = await open({
            multiple: false,
            directory: isDirectory,
            title: type,
        });
        if (path) {
            field.onChange(path);
        }
    }

    const progressPercentage = useMemo(
        () =>
            totalJobs > 0 ? Math.round((completedJobs / totalJobs) * 100) : 0,
        [completedJobs, totalJobs],
    );

    return (
        <div className="flex min-h-svh w-full items-center justify-center p-6 md:p-10">
            <div className="grid gap-6 w-full max-w-4xl">
                <div className="flex justify-end">
                    <Button
                        variant="outline"
                        size="icon"
                        onClick={() =>
                            setTheme(theme === 'dark' ? 'light' : 'dark')
                        }
                        aria-label="Toggle theme"
                    >
                        {theme === 'dark' ? (
                            <Sun className="h-5 w-5" />
                        ) : (
                            <Moon className="h-5 w-5" />
                        )}
                    </Button>
                </div>
                <Card>
                    <CardHeader>
                        <CardTitle>Puppeteer Configuration</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <Form {...form}>
                            <form className="space-y-6">
                                <div className="space-y-8">
                                    {/* Browser Configuration Section */}
                                    <div className="space-y-4">
                                        <FormField
                                            control={form.control}
                                            name="chrome_path"
                                            render={({ field }) => (
                                                <FormItem>
                                                    <FormLabel>
                                                        Chrome Path
                                                    </FormLabel>
                                                    <div className="flex gap-2">
                                                        <FormControl>
                                                            <Input
                                                                placeholder="/usr/bin/google-chrome"
                                                                {...field}
                                                            />
                                                        </FormControl>
                                                        <Button
                                                            type="button"
                                                            variant="outline"
                                                            size="icon"
                                                            onClick={() =>
                                                                handleFilePicker(
                                                                    field,
                                                                    'Chrome executable',
                                                                )
                                                            }
                                                        >
                                                            <FolderOpen className="h-4 w-4" />
                                                        </Button>
                                                    </div>
                                                    <FormDescription>
                                                        Path to Chrome
                                                        executable
                                                    </FormDescription>
                                                    <FormMessage />
                                                </FormItem>
                                            )}
                                        />

                                        <FormField
                                            control={form.control}
                                            name="user_data_dir"
                                            render={({ field }) => (
                                                <FormItem>
                                                    <FormLabel>
                                                        User Data Directory
                                                    </FormLabel>
                                                    <div className="flex gap-2">
                                                        <FormControl>
                                                            <Input
                                                                placeholder="/path/to/user/data"
                                                                {...field}
                                                            />
                                                        </FormControl>
                                                        <Button
                                                            type="button"
                                                            variant="outline"
                                                            size="icon"
                                                            onClick={() =>
                                                                handleFilePicker(
                                                                    field,
                                                                    'User data directory',
                                                                    true,
                                                                )
                                                            }
                                                        >
                                                            <FolderOpen className="h-4 w-4" />
                                                        </Button>
                                                    </div>
                                                    <FormDescription>
                                                        Path to Chrome user data
                                                    </FormDescription>
                                                    <FormMessage />
                                                </FormItem>
                                            )}
                                        />

                                        <FormField
                                            control={form.control}
                                            name="headless"
                                            render={({ field }) => (
                                                <FormItem className="flex flex-row items-center justify-between rounded-lg border p-4">
                                                    <div className="space-y-0.5">
                                                        <FormLabel className="text-base">
                                                            Headless Mode
                                                        </FormLabel>
                                                        <FormDescription>
                                                            Run browser without
                                                            visible UI
                                                        </FormDescription>
                                                    </div>
                                                    <div className="flex items-center space-x-2">
                                                        <FormControl>
                                                            <Switch
                                                                checked={
                                                                    field.value
                                                                }
                                                                onCheckedChange={
                                                                    field.onChange
                                                                }
                                                            />
                                                        </FormControl>
                                                        {field.value ? (
                                                            <EyeOff className="h-4 w-4 text-muted-foreground" />
                                                        ) : (
                                                            <Monitor className="h-4 w-4 text-muted-foreground" />
                                                        )}
                                                    </div>
                                                </FormItem>
                                            )}
                                        />
                                    </div>

                                    {/* Automation Settings Section */}
                                    <div>
                                        <h3 className="text-lg font-medium mb-4">
                                            Automation Settings
                                        </h3>
                                        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                                            <FormField
                                                control={form.control}
                                                name="wait_for_navigation"
                                                render={({ field }) => (
                                                    <FormItem>
                                                        <FormLabel>
                                                            Navigation Timeout
                                                            (s)
                                                        </FormLabel>
                                                        <FormControl>
                                                            <Input
                                                                type="number"
                                                                step={1}
                                                                {...field}
                                                                onChange={(e) =>
                                                                    field.onChange(
                                                                        Number(
                                                                            e
                                                                                .target
                                                                                .value,
                                                                        ),
                                                                    )
                                                                }
                                                            />
                                                        </FormControl>
                                                        <FormDescription>
                                                            Time to wait for
                                                            page navigation
                                                        </FormDescription>
                                                        <FormMessage />
                                                    </FormItem>
                                                )}
                                            />

                                            <FormField
                                                control={form.control}
                                                name="max_retries"
                                                render={({ field }) => (
                                                    <FormItem>
                                                        <FormLabel>
                                                            Max Retries
                                                        </FormLabel>
                                                        <FormControl>
                                                            <Input
                                                                type="number"
                                                                min={0}
                                                                max={10}
                                                                {...field}
                                                                onChange={(e) =>
                                                                    field.onChange(
                                                                        Number(
                                                                            e
                                                                                .target
                                                                                .value,
                                                                        ),
                                                                    )
                                                                }
                                                            />
                                                        </FormControl>
                                                        <FormDescription>
                                                            Number of times to
                                                            retry on failure
                                                        </FormDescription>
                                                        <FormMessage />
                                                    </FormItem>
                                                )}
                                            />

                                            <FormField
                                                control={form.control}
                                                name="tab_count"
                                                render={({ field }) => (
                                                    <FormItem>
                                                        <FormLabel>
                                                            Tab Count
                                                        </FormLabel>
                                                        <FormControl>
                                                            <Input
                                                                type="number"
                                                                min={1}
                                                                {...field}
                                                                onChange={(e) =>
                                                                    field.onChange(
                                                                        Number(
                                                                            e
                                                                                .target
                                                                                .value,
                                                                        ),
                                                                    )
                                                                }
                                                            />
                                                        </FormControl>
                                                        <FormDescription>
                                                            Number of tabs to
                                                            open
                                                        </FormDescription>
                                                        <FormMessage />
                                                    </FormItem>
                                                )}
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div className="flex gap-2">
                                    <Button
                                        type="button"
                                        variant="outline"
                                        onClick={saveConfiguration}
                                        className="gap-2"
                                        disabled={status === 'running'}
                                    >
                                        <Save className="w-4 h-4" />
                                        Save Config
                                    </Button>

                                    {isStoppable ? (
                                        <Button
                                            type="button"
                                            variant="destructive"
                                            onClick={stopAutomation}
                                            className="gap-2 ml-auto"
                                        >
                                            <Square className="w-4 h-4" />
                                            Stop
                                        </Button>
                                    ) : (
                                        <Button
                                            type="button"
                                            variant="secondary"
                                            onClick={runAutomation}
                                            className="gap-2 ml-auto"
                                            disabled={status === 'running'}
                                        >
                                            {status === 'running' ? (
                                                <Loader2 className="w-4 h-4 animate-spin" />
                                            ) : (
                                                <Play className="w-4 h-4" />
                                            )}
                                            Run Now
                                        </Button>
                                    )}
                                </div>
                            </form>
                        </Form>
                    </CardContent>
                </Card>

                {(status === 'running' ||
                    status === 'completed' ||
                    status === 'stopped') && (
                    <Card>
                        <CardContent className="pt-6">
                            <div className="flex items-center justify-between mb-2">
                                <div className="flex items-center gap-2">
                                    {status === 'running' ? (
                                        <>
                                            <Loader2 className="h-5 w-5 text-blue-500 animate-spin" />
                                            <span className="font-medium text-blue-700 dark:text-blue-400">
                                                Processing automation...
                                            </span>
                                        </>
                                    ) : status === 'completed' ? (
                                        <>
                                            <CheckCircle2 className="h-5 w-5 text-green-500" />
                                            <span className="font-medium text-green-700 dark:text-green-400">
                                                Automation completed
                                            </span>
                                        </>
                                    ) : (
                                        <>
                                            <XCircle className="h-5 w-5 text-amber-500" />
                                            <span className="font-medium text-amber-700 dark:text-amber-400">
                                                Automation stopped
                                            </span>
                                        </>
                                    )}
                                </div>
                                <div className="text-sm font-medium">
                                    {completedJobs} of {totalJobs} jobs
                                </div>
                            </div>

                            <div className="relative pt-1">
                                <div className="overflow-hidden h-2 text-xs flex rounded bg-gray-200 dark:bg-gray-700">
                                    <div
                                        style={{
                                            width: `${progressPercentage}%`,
                                        }}
                                        className={`shadow-none flex flex-col text-center whitespace-nowrap text-white justify-center transition-all duration-500 ${
                                            status === 'completed'
                                                ? 'bg-green-500'
                                                : status === 'stopped'
                                                  ? 'bg-amber-500'
                                                  : 'bg-blue-500'
                                        }`}
                                    ></div>
                                </div>
                            </div>

                            {status !== 'running' ? (
                                <div className="flex justify-between mt-1 text-xs text-gray-500 dark:text-gray-400">
                                    <div className="flex items-center gap-1">
                                        <Clock className="h-3 w-3" />
                                        {status === 'stopped'
                                            ? `Stopped after ${convert(elapsed)}`
                                            : `Completed in ${convert(elapsed)}`}
                                    </div>
                                    <div>{progressPercentage}%</div>
                                </div>
                            ) : (
                                <div></div>
                            )}
                        </CardContent>
                    </Card>
                )}
            </div>
        </div>
    );
}

export default App;
