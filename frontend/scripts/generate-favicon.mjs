import { readFileSync, writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import path from 'path';
import sharp from 'sharp';
import toIco from 'to-ico';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const publicDir = path.join(__dirname, '..', 'public');
const svgPath = path.join(publicDir, 'favicon.svg');
const icoPath = path.join(publicDir, 'favicon.ico');

const svg = readFileSync(svgPath);
const png32 = await sharp(svg).resize(32, 32).png().toBuffer();
const ico = await toIco([png32]);
writeFileSync(icoPath, ico);
console.log('Generated public/favicon.ico');
