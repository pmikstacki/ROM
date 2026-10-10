import {test} from 'node:test';
import assert from 'node:assert/strict';
import {requirePrivateTrustNamespace,requireReadOnlyTrustMount} from './trust-namespace.mjs';
test('trust namespace requires a different actual namespace and private root propagation',()=>{
 assert.doesNotThrow(()=>requirePrivateTrustNamespace('mnt:[1]','mnt:[2]','1 0 0:1 / / rw - ext4 /dev/root rw'));
 assert.throws(()=>requirePrivateTrustNamespace('mnt:[1]','mnt:[1]','1 0 0:1 / / rw - ext4 /dev/root rw'));
 assert.throws(()=>requirePrivateTrustNamespace('mnt:[1]','mnt:[2]','1 0 0:1 / / rw shared:1 - ext4 /dev/root rw'));
});
test('trust mount admission requires exact read-only mount and rejects missing/shared/writable mount',()=>{
 assert.doesNotThrow(()=>requireReadOnlyTrustMount('2 1 0:2 /private/trust /etc/ssl/certs ro - ext4 /dev/loop2 rw'));
 for(const raw of ['','2 1 0:2 /private/trust /etc/ssl/certs rw - ext4 /dev/loop2 rw','2 1 0:2 /private/trust /etc/ssl/certs ro shared:2 - ext4 /dev/loop2 rw'])assert.throws(()=>requireReadOnlyTrustMount(raw));
});
