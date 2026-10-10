// Coordinator-approved two-library supplement. This is not whole-runtime admission.
export const cryptoPackages=Object.freeze([
 Object.freeze({name:'libgnutls30t64',source:'gnutls28',directory:'g/gnutls28',version:'3.8.9-3+deb13u4',bytes:1469024,sha256:'18a8bdfd91c7e3bcb01719d55a2b56849c7160b34f1f52b1c4fdfdd41bf1352b',soname:'libgnutls.so.30'}),
 Object.freeze({name:'libtasn1-6',source:'libtasn1-6',directory:'libt/libtasn1-6',version:'4.20.0-2+deb13u1',bytes:50112,sha256:'23fec6e06583ce2bad9b2c04c9b485e90440e259b1abf8677cd80d3ce60831ad',soname:'libtasn1.so.6'}),
]);
export function selectCryptoLibrary(members,profile){
 if(!cryptoPackages.includes(profile)||!(members instanceof Map))throw Error('unapproved crypto projection');
 const filename=profile.soname,name=`usr/lib/x86_64-linux-gnu/${filename}`;
 let member=members.get(name),source=name;
 if(member?.type==='2'){
  if(typeof member.link!=='string'||!member.link.startsWith(`${name}.`)||!/^[0-9.]+$/.test(member.link.slice(name.length+1)))throw Error('crypto SONAME target rejected');
  source=member.link;member=members.get(source);
 }
 if(member?.type!=='0'||!Buffer.isBuffer(member.bytes)||member.bytes.length===0||member.bytes.length>8*1024**2)throw Error('regular bounded crypto library required');
 return{filename,source,bytes:member.bytes};
}
