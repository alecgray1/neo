#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import prompts from 'prompts';
import { blue, green, yellow, red, cyan, dim } from 'kolorist';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Default config options for different plugin types
const CONFIG_PRESETS = {
    'data-service': {
        poll_interval_seconds: 30,
    },
    'integration': {
        api_url: 'https://api.example.com',
        api_key: '',
    },
    'automation': {
        enabled: true,
    },
    'empty': {},
};

async function main() {
    console.log();
    console.log(blue('  Neo Plugin Creator'));
    console.log(dim('  Scaffold a new Neo BMS plugin'));
    console.log();

    // Get project name from args
    let targetDir = process.argv[2];
    let template = process.argv[3];

    const response = await prompts([
        {
            type: targetDir ? null : 'text',
            name: 'projectName',
            message: 'Plugin name:',
            initial: 'my-neo-plugin',
            validate: (value) => {
                if (!value) return 'Plugin name is required';
                if (!/^[a-z0-9-]+$/.test(value)) {
                    return 'Plugin name must be lowercase with hyphens only';
                }
                return true;
            },
        },
        {
            type: 'text',
            name: 'description',
            message: 'Description:',
            initial: 'A Neo BMS plugin',
        },
        {
            type: template ? null : 'select',
            name: 'template',
            message: 'Template:',
            choices: [
                { title: 'JavaScript', value: 'javascript', description: 'Simple JS plugin' },
                { title: 'TypeScript', value: 'typescript', description: 'TypeScript with build step' },
            ],
        },
        {
            type: 'select',
            name: 'pluginType',
            message: 'Plugin type:',
            choices: [
                { title: 'Data Service', value: 'data-service', description: 'Fetches/generates data periodically' },
                { title: 'Integration', value: 'integration', description: 'Integrates with external APIs' },
                { title: 'Automation', value: 'automation', description: 'Reacts to events and automates' },
                { title: 'Empty', value: 'empty', description: 'Minimal starting point' },
            ],
        },
        {
            type: 'multiselect',
            name: 'subscriptions',
            message: 'Subscribe to events:',
            choices: [
                { title: 'All events (*)', value: '*' },
                { title: 'PointValueChanged', value: 'PointValueChanged' },
                { title: 'AlarmRaised', value: 'AlarmRaised' },
                { title: 'AlarmCleared', value: 'AlarmCleared' },
                { title: 'DeviceDiscovered', value: 'DeviceDiscovered' },
                { title: 'DeviceStatusChanged', value: 'DeviceStatusChanged' },
            ],
            initial: (prev) => prev === 'automation' ? [1, 2] : [0],
            hint: '- Space to select, Enter to confirm',
        },
    ], {
        onCancel: () => {
            console.log(red('\n  Cancelled\n'));
            process.exit(1);
        }
    });

    targetDir = targetDir || response.projectName;
    template = template || response.template;

    const root = path.resolve(process.cwd(), targetDir);
    const pluginId = path.basename(targetDir);

    // Check if directory exists
    if (fs.existsSync(root) && fs.readdirSync(root).length > 0) {
        const { overwrite } = await prompts({
            type: 'confirm',
            name: 'overwrite',
            message: `Directory ${yellow(targetDir)} is not empty. Overwrite?`,
            initial: false,
        });
        if (!overwrite) {
            console.log(red('\n  Cancelled\n'));
            process.exit(1);
        }
        // Clear directory
        fs.rmSync(root, { recursive: true });
    }

    // Create directory
    fs.mkdirSync(root, { recursive: true });

    console.log();
    console.log(`  Creating plugin in ${green(root)}`);
    console.log();

    // Copy template files
    const templateDir = path.join(__dirname, 'templates', template);
    copyDir(templateDir, root);

    // Generate neo-plugin.json
    const subscriptions = response.subscriptions.length > 0
        ? response.subscriptions
        : ['*'];

    const manifest = {
        id: pluginId,
        name: toTitleCase(pluginId),
        description: response.description,
        version: '1.0.0',
        main: template === 'typescript' ? 'dist/index.js' : 'src/index.js',
        config: CONFIG_PRESETS[response.pluginType] || {},
        subscriptions,
    };

    fs.writeFileSync(
        path.join(root, 'neo-plugin.json'),
        JSON.stringify(manifest, null, 4) + '\n'
    );

    // Customize the template based on plugin type
    customizeTemplate(root, template, response.pluginType, pluginId);

    // Print success message
    console.log(green('  Done!') + ' Created plugin ' + cyan(pluginId));
    console.log();
    console.log('  Next steps:');
    console.log();
    console.log(`    ${dim('$')} cd ${yellow(targetDir)}`);

    if (template === 'typescript') {
        console.log(`    ${dim('$')} ${yellow('npm install')}`);
        console.log(`    ${dim('$')} ${yellow('npm run build')}`);
    }

    console.log();
    console.log(`  To use the plugin:`);
    console.log();
    console.log(`    Copy the ${cyan(targetDir)} folder to your Neo ${cyan('plugins/')} directory`);
    console.log(`    and restart Neo.`);
    console.log();
}

function copyDir(src, dest) {
    fs.mkdirSync(dest, { recursive: true });
    for (const file of fs.readdirSync(src)) {
        const srcFile = path.join(src, file);
        const destFile = path.join(dest, file);
        const stat = fs.statSync(srcFile);
        if (stat.isDirectory()) {
            copyDir(srcFile, destFile);
        } else {
            fs.copyFileSync(srcFile, destFile);
        }
    }
}

function toTitleCase(str) {
    return str
        .replace(/-/g, ' ')
        .replace(/\b\w/g, c => c.toUpperCase());
}

function customizeTemplate(root, template, pluginType, pluginId) {
    const ext = template === 'typescript' ? 'ts' : 'js';
    const srcFile = path.join(root, 'src', `index.${ext}`);

    let content = fs.readFileSync(srcFile, 'utf-8');

    // Replace placeholder with plugin ID
    content = content.replace(/{{PLUGIN_ID}}/g, pluginId);
    content = content.replace(/{{PLUGIN_NAME}}/g, toTitleCase(pluginId));

    // Add plugin-type specific code
    if (pluginType === 'data-service') {
        content = content.replace('// {{INTERVAL_CODE}}', `
    // Start periodic data fetch
    const interval = config.poll_interval_seconds || 30;
    intervalId = setInterval(() => {
        fetchData();
    }, interval * 1000);

    // Initial fetch
    fetchData();`);

        content = content.replace('// {{HELPER_FUNCTIONS}}', `
function fetchData() {
    // TODO: Implement your data fetching logic
    const value = Math.random() * 100;

    // Write to virtual point
    Neo.points.write("virtual/${pluginId}/value", { Real: value });

    // Publish event
    Neo.events.publish({
        type: "DataUpdated",
        source: "${pluginId}",
        data: { value },
    });

    Neo.log.info(\`Data updated: \${value.toFixed(2)}\`);
}`);
    } else if (pluginType === 'integration') {
        content = content.replace('// {{INTERVAL_CODE}}', `
    // Validate config
    if (!config.api_url) {
        Neo.log.warn("No api_url configured");
    }`);

        content = content.replace('// {{HELPER_FUNCTIONS}}', `
async function callExternalApi(endpoint, data) {
    // TODO: Implement API calls
    // Note: fetch() is not available in Neo plugins yet
    // This is a placeholder for future HTTP support
    Neo.log.info(\`Would call API: \${endpoint}\`);
    return { success: true };
}`);
    } else if (pluginType === 'automation') {
        content = content.replace('// {{INTERVAL_CODE}}', `
    if (!config.enabled) {
        Neo.log.warn("Automation is disabled in config");
    }`);

        content = content.replace('// {{EVENT_HANDLER}}', `
        // React to point value changes
        if (event.type === 'PointValueChanged') {
            handlePointChange(event);
        }

        // React to alarms
        if (event.type === 'AlarmRaised') {
            handleAlarm(event);
        }`);

        content = content.replace('// {{HELPER_FUNCTIONS}}', `
function handlePointChange(event) {
    const { point_id, new_value } = event;
    Neo.log.debug(\`Point changed: \${point_id} = \${JSON.stringify(new_value)}\`);

    // TODO: Add your automation logic
    // Example: if temperature > threshold, do something
}

function handleAlarm(event) {
    const { alarm_id, severity } = event;
    Neo.log.info(\`Alarm raised: \${alarm_id} (severity: \${severity})\`);

    // TODO: Add your alarm handling logic
}`);
    }

    // Clean up unused placeholders
    content = content.replace(/\/\/ \{\{.*?\}\}\n?/g, '');

    fs.writeFileSync(srcFile, content);
}

main().catch((err) => {
    console.error(red('Error:'), err.message);
    process.exit(1);
});
