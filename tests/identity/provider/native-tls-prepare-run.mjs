import { readNativeTlsJson, nativeTlsPrivatePath } from './native-tls-io.mjs';
import { prepareNativeTls } from './native-tls-prepare.mjs';
try {
  const options = readNativeTlsJson(nativeTlsPrivatePath(process.argv[2]), 16384);
  const result = prepareNativeTls(options);
  console.log(JSON.stringify({ status: result.status, preparation: `${result.directory}/preparation.json`, release_admission: false }));
} catch { process.exitCode = 1; console.error('native TLS preparation failed'); }
