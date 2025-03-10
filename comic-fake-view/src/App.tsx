import { useMemo, useState } from 'react';
import './index.css';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import {
    Save,
    FolderOpen,
    Play,
    RotateCcw,
    CheckCircle2,
    Clock,
    Loader2,
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

const formSchema = z.object({
    chromePath: z.string().optional(),
    userDataDir: z.string().optional(),
    waitForNavigation: z.number(),
    maxRetries: z.number().min(0).max(10),
    tabCount: z.number().min(1),
});

type FormValues = z.infer<typeof formSchema>;

const defaultValues: FormValues = {
    chromePath: undefined,
    userDataDir: undefined,
    waitForNavigation: 5,
    maxRetries: 3,
    tabCount: 5
};

function App() {
    const [totalJobs, setTotalJobs] = useState(10);
    const [completedJobs, setCompletedJobs] = useState(0);
    const [status, setStatus] = useState<'idle' | 'running' | 'completed'>(
        'idle',
    );

    const form = useForm<FormValues>({
        resolver: zodResolver(formSchema),
        defaultValues,
    });

    const onSubmit = async (data: FormValues) => {
        await invoke('set_config', {
            config: data,
        });
    };

    const runAutomation = async () => {
        await invoke('set_config', {
            config: form.getValues(),
        });
        invoke('run');
        once('total_jobs', async (event) => {
            setTotalJobs(Number(event.payload));
            setStatus('running');
            setCompletedJobs(0);

            const unlisten = await listen('complete', () =>
                setCompletedJobs((prev) => prev + 1),
            );

            await once('completed', async () => {
                unlisten();
                setStatus('completed');
            });
        });
    };

    // This would be implemented with actual file system access in a desktop app
    async function handleFilePicker(
        field: any,
        type: string,
        isDirectory: boolean = false,
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
        <div className="grid gap-6">
            <Card>
                <CardHeader>
                    <CardTitle>Puppeteer Configuration</CardTitle>
                </CardHeader>
                <CardContent>
                    <Form {...form}>
                        <form
                            onSubmit={form.handleSubmit(onSubmit)}
                            className="space-y-6"
                        >
                            <div className="grid gap-6">
                                <FormField
                                    control={form.control}
                                    name="chromePath"
                                    render={({ field }) => (
                                        <FormItem>
                                            <FormLabel>Chrome Path</FormLabel>
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
                                                Path to Chrome executable
                                            </FormDescription>
                                            <FormMessage />
                                        </FormItem>
                                    )}
                                />

                                <FormField
                                    control={form.control}
                                    name="userDataDir"
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
                                                Path to Chrome user data (for
                                                saved sessions)
                                            </FormDescription>
                                            <FormMessage />
                                        </FormItem>
                                    )}
                                />

                                <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                                    <FormField
                                        control={form.control}
                                        name="waitForNavigation"
                                        render={({ field }) => (
                                            <FormItem>
                                                <FormLabel>
                                                    Navigation Timeout (s)
                                                </FormLabel>
                                                <FormControl>
                                                    <Input
                                                        type="number"
                                                        step={1}
                                                        {...field}
                                                        onChange={(e) =>
                                                            field.onChange(
                                                                Number(
                                                                    e.target
                                                                        .value,
                                                                ),
                                                            )
                                                        }
                                                    />
                                                </FormControl>
                                                <FormDescription>
                                                    Time to wait for page
                                                    navigation
                                                </FormDescription>
                                                <FormMessage />
                                            </FormItem>
                                        )}
                                    />

                                    <FormField
                                        control={form.control}
                                        name="maxRetries"
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
                                                                    e.target
                                                                        .value,
                                                                ),
                                                            )
                                                        }
                                                    />
                                                </FormControl>
                                                <FormDescription>
                                                    Number of times to retry on
                                                    failure
                                                </FormDescription>
                                                <FormMessage />
                                            </FormItem>
                                        )}
                                    />

                                    <FormField
                                        control={form.control}
                                        name="tabCount"
                                        render={({ field }) => (
                                            <FormItem>
                                                <FormLabel>
                                                    Tab Count
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
                                                                    e.target
                                                                        .value,
                                                                ),
                                                            )
                                                        }
                                                    />
                                                </FormControl>
                                                <FormDescription>
                                                    Number of tab to open
                                                </FormDescription>
                                                <FormMessage />
                                            </FormItem>
                                        )}
                                    />
                                </div>
                            </div>

                            <div className="flex gap-2">
                                <Button type="submit" className="gap-2">
                                    <Save className="w-4 h-4" />
                                    Save Configuration
                                </Button>
                                <Button
                                    type="button"
                                    variant="outline"
                                    onClick={() => form.reset(defaultValues)}
                                    className="gap-2"
                                >
                                    <RotateCcw className="w-4 h-4" />
                                    Reset
                                </Button>
                                <Button
                                    type="button"
                                    variant="secondary"
                                    onClick={runAutomation}
                                    className="gap-2"
                                    disabled={status == 'running'}
                                >
                                    {status == 'running' ? (
                                        <Loader2 className="w-4 h-4 animate-spin" />
                                    ) : (
                                        <Play className="w-4 h-4" />
                                    )}
                                    Run Now
                                </Button>
                            </div>
                        </form>
                    </Form>
                </CardContent>
            </Card>

            {(status === 'running' || status === 'completed') && (
                <Card>
                    <CardContent className="pt-6">
                        <div className="flex items-center justify-between mb-2">
                            <div className="flex items-center gap-2">
                                {status === 'running' ? (
                                    <>
                                        <Loader2 className="h-5 w-5 text-blue-500 animate-spin" />
                                        <span className="font-medium text-blue-700">
                                            Processing automation...
                                        </span>
                                    </>
                                ) : (
                                    <>
                                        <CheckCircle2 className="h-5 w-5 text-green-500" />
                                        <span className="font-medium text-green-700">
                                            Automation completed
                                        </span>
                                    </>
                                )}
                            </div>
                            <div className="text-sm font-medium">
                                {completedJobs} of {totalJobs} jobs
                            </div>
                        </div>

                        <div className="relative pt-1">
                            <div className="overflow-hidden h-2 text-xs flex rounded bg-gray-200">
                                <div
                                    style={{ width: `${progressPercentage}%` }}
                                    className={`shadow-none flex flex-col text-center whitespace-nowrap text-white justify-center transition-all duration-500 ${
                                        status === 'completed'
                                            ? 'bg-green-500'
                                            : 'bg-blue-500'
                                    }`}
                                ></div>
                            </div>
                        </div>

                        <div className="flex justify-between mt-1 text-xs text-gray-500">
                            <div className="flex items-center gap-1">
                                <Clock className="h-3 w-3" />
                                {status === 'completed'
                                    ? `Completed in ${(totalJobs * form.getValues('waitForNavigation')).toFixed(1)}s`
                                    : 'Estimated time remaining: ' +
                                      (
                                          (totalJobs - completedJobs) *
                                          form.getValues('waitForNavigation')
                                      ).toFixed(1) +
                                      's'}
                            </div>
                            <div>{progressPercentage}%</div>
                        </div>
                    </CardContent>
                </Card>
            )}
        </div>
    );
}

export default App;
