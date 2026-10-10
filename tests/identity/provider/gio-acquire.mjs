import { acquirePrivateGio } from './gio-acquisition.mjs';
try { await acquirePrivateGio(); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'gio-prerequisite-failed', production_dependency_admission: false })); }
