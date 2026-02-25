#!/usr/bin/env node

/**
 * Contract Documentation Generator
 * 
 * This script generates documentation from contract manifests.
 * It can create:
 * - Markdown documentation
 * - Type definitions (TypeScript)
 * - API client stubs
 * - Deployment scripts
 */

const fs = require('fs');
const path = require('path');

// Configuration
const CONTRACTS_DIR = path.join(__dirname, '..');
const OUTPUT_DIR = path.join(__dirname, '..', 'generated-docs');
const MANIFESTS = [
  'bounty-escrow-manifest.json',
  'grainlify-core-manifest.json',
  'program-escrow-manifest.json'
];

// Ensure output directory exists
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

/**
 * Load all manifests
 */
function loadManifests() {
  const manifests = {};
  
  MANIFESTS.forEach(filename => {
    const filePath = path.join(CONTRACTS_DIR, filename);
    if (fs.existsSync(filePath)) {
      const content = fs.readFileSync(filePath, 'utf8');
      manifests[filename] = JSON.parse(content);
    }
  });
  
  return manifests;
}

/**
 * Generate Markdown documentation
 */
function generateMarkdown(manifests) {
  let markdown = '# Smart Contract API Documentation\n\n';
  markdown += 'Generated from contract manifests. Do not edit manually.\n\n';
  markdown += `Generated on: ${new Date().toISOString()}\n\n`;
  
  Object.entries(manifests).forEach(([filename, manifest]) => {
    const contractName = manifest.contract_name;
    const version = manifest.version.current;
    const purpose = manifest.contract_purpose;
    
    markdown += `## ${contractName}\n\n`;
    markdown += `**Version:** ${version}\n\n`;
    markdown += `**Purpose:** ${purpose}\n\n`;
    
    // Entrypoints
    markdown += '### Entrypoints\n\n';
    
    // Public functions
    if (manifest.entrypoints.public && manifest.entrypoints.public.length > 0) {
      markdown += '#### Public Functions\n\n';
      manifest.entrypoints.public.forEach(fn => {
        markdown += `##### ${fn.name}\n\n`;
        markdown += `${fn.description}\n\n`;
        
        if (fn.parameters && fn.parameters.length > 0) {
          markdown += '**Parameters:**\n\n';
          markdown += '| Name | Type | Description | Optional |\n';
          markdown += '|------|------|-------------|----------|\n';
          
          fn.parameters.forEach(param => {
            const optional = param.optional ? 'Yes' : 'No';
            markdown += `| ${param.name} | ${param.type} | ${param.description} | ${optional} |\n`;
          });
          markdown += '\n';
        }
        
        markdown += `**Authorization:** ${fn.authorization}\n\n`;
        markdown += `**Gas Estimate:** ${fn.gas_estimate}\n\n`;
        
        if (fn.pausable) {
          markdown += '**Pausable:** Yes\n\n';
        }
      });
    }
    
    // View functions
    if (manifest.entrypoints.view && manifest.entrypoints.view.length > 0) {
      markdown += '#### View Functions\n\n';
      manifest.entrypoints.view.forEach(fn => {
        markdown += `##### ${fn.name}\n\n`;
        markdown += `${fn.description}\n\n`;
        
        if (fn.parameters && fn.parameters.length > 0) {
          markdown += '**Parameters:**\n\n';
          markdown += '| Name | Type | Description | Optional |\n';
          markdown += '|------|------|-------------|----------|\n';
          
          fn.parameters.forEach(param => {
            const optional = param.optional ? 'Yes' : 'No';
            markdown += `| ${param.name} | ${param.type} | ${param.description} | ${optional} |\n`;
          });
          markdown += '\n';
        }
        
        markdown += `**Returns:** ${fn.returns.type} - ${fn.returns.description}\n\n`;
        markdown += `**Gas Estimate:** ${fn.gas_estimate}\n\n`;
      });
    }
    
    // Admin functions
    if (manifest.entrypoints.admin && manifest.entrypoints.admin.length > 0) {
      markdown += '#### Admin Functions\n\n';
      manifest.entrypoints.admin.forEach(fn => {
        markdown += `##### ${fn.name}\n\n`;
        markdown += `${fn.description}\n\n`;
        
        if (fn.parameters && fn.parameters.length > 0) {
          markdown += '**Parameters:**\n\n';
          markdown += '| Name | Type | Description | Optional |\n';
          markdown += '|------|------|-------------|----------|\n';
          
          fn.parameters.forEach(param => {
            const optional = param.optional ? 'Yes' : 'No';
            markdown += `| ${param.name} | ${param.type} | ${param.description} | ${optional} |\n`;
          });
          markdown += '\n';
        }
        
        markdown += `**Authorization:** ${fn.authorization}\n\n`;
        markdown += `**Gas Estimate:** ${fn.gas_estimate}\n\n`;
      });
    }
    
    // Configuration
    if (manifest.configuration && manifest.configuration.parameters) {
      markdown += '### Configuration Parameters\n\n';
      markdown += '| Name | Type | Default | Description | Admin Only |\n';
      markdown += '|------|------|---------|-------------|------------|\n';
      
      manifest.configuration.parameters.forEach(param => {
        const adminOnly = param.admin_only ? 'Yes' : 'No';
        const defaultValue = JSON.stringify(param.default_value);
        markdown += `| ${param.name} | ${param.type} | ${defaultValue} | ${param.description} | ${adminOnly} |\n`;
      });
      markdown += '\n';
    }
    
    // Security Features
    if (manifest.behaviors && manifest.behaviors.security_features) {
      markdown += '### Security Features\n\n';
      manifest.behaviors.security_features.forEach(feature => {
        markdown += `- ${feature}\n`;
      });
      markdown += '\n';
    }
    
    // Events
    if (manifest.behaviors && manifest.behaviors.events) {
      markdown += '### Events\n\n';
      manifest.behaviors.events.forEach(event => {
        markdown += `#### ${event.name}\n\n`;
        markdown += `${event.description}\n\n`;
        markdown += `**Trigger:** ${event.trigger}\n\n`;
        
        if (event.data && event.data.length > 0) {
          markdown += '**Data:**\n\n';
          markdown += '| Name | Type | Description |\n';
          markdown += '|------|------|-------------|\n';
          
          event.data.forEach(field => {
            markdown += `| ${field.name} | ${field.type} | ${field.description} |\n`;
          });
          markdown += '\n';
        }
      });
    }
    
    markdown += '---\n\n';
  });
  
  return markdown;
}

/**
 * Generate TypeScript type definitions
 */
function generateTypeScript(manifests) {
  let types = `// Generated TypeScript definitions for Grainlify contracts
// Generated on: ${new Date().toISOString()}
// Do not edit manually

`;
  
  Object.entries(manifests).forEach(([filename, manifest]) => {
    const contractName = manifest.contract_name;
    const safeName = contractName.replace(/[^a-zA-Z0-9]/g, '');
    
    types += `// ${contractName} - v${manifest.version.current}\n`;
    types += `// ${manifest.contract_purpose}\n\n`;
    
    // Entrypoint types
    if (manifest.entrypoints.public) {
      types += `export namespace ${safeName} {\n`;
      
      manifest.entrypoints.public.forEach(fn => {
        // Function signature type
        const paramTypes = fn.parameters.map(p => {
          const optional = p.optional ? '?' : '';
          return `${p.name}${optional}: ${p.type}`;
        }).join(', ');
        
        const returnType = fn.returns.type;
        types += `  export type ${fn.name}Params = { ${paramTypes} };\n`;
        types += `  export type ${fn.name}Result = ${returnType};\n`;
        types += `  export declare function ${fn.name}(params: ${fn.name}Params): Promise<${fn.name}Result>;\n\n`;
      });
      
      types += `}\n\n`;
    }
  });
  
  return types;
}

/**
 * Generate deployment scripts
 */
function generateDeploymentScripts(manifests) {
  let scripts = `#!/bin/bash
# Generated deployment scripts for Grainlify contracts
# Generated on: ${new Date().toISOString()}
# Do not edit manually

set -e

echo "🚀 Grainlify Contract Deployment Scripts"
echo "====================================="

`;
  
  Object.entries(manifests).forEach(([filename, manifest]) => {
    const contractName = manifest.contract_name;
    const safeName = contractName.replace(/[^a-zA-Z0-9]/g, '').toLowerCase();
    
    scripts += `# ${contractName} Deployment
deploy_${safeName}() {
    echo "📦 Deploying ${contractName}..."
    
    # Build contract
    echo "Building contract..."
    # Add build commands here
    
    # Deploy to testnet first
    echo "Deploying to testnet..."
    # stellar contract deploy --network testnet --wasm target/wasm32-unknown-unknown/release/${safeName}.wasm
    
    # Initialize contract
    echo "Initializing contract..."
`;
    
    // Add initialization commands based on manifest
    if (manifest.entrypoints.admin && manifest.entrypoints.admin.length > 0) {
      const initFn = manifest.entrypoints.admin.find(fn => fn.name.includes('init'));
      if (initFn) {
        scripts += `    # stellar contract invoke --id CONTRACT_ID -- ${initFn.name} \\
`;
        initFn.parameters.forEach(param => {
          scripts += `    #   --${param.name} ${param.name.toUpperCase()} \\\n`;
        });
        scripts += '\n';
      }
    }
    
    scripts += `    echo "✅ ${contractName} deployed and initialized"
}

`;
  });
  
  // Add main deployment function
  scripts += `# Deploy all contracts
deploy_all() {
    echo "🚀 Deploying all contracts..."
`;
  
  Object.keys(manifests).forEach(filename => {
    const contractName = manifests[filename].contract_name;
    const safeName = contractName.replace(/[^a-zA-Z0-9]/g, '').toLowerCase();
    scripts += `    deploy_${safeName}\n`;
  });
  
  scripts += `    echo "✅ All contracts deployed"
}

# Show usage
usage() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
`;
  
  Object.keys(manifests).forEach(filename => {
    const contractName = manifests[filename].contract_name;
    const safeName = contractName.replace(/[^a-zA-Z0-9]/g, '').toLowerCase();
    scripts += `    echo "  ${safeName}     Deploy ${contractName}"\n`;
  });
  
  scripts += `    echo "  all         Deploy all contracts"
    echo ""
}

# Main script logic
case "\${1:-}" in
`;
  
  Object.keys(manifests).forEach(filename => {
    const contractName = manifests[filename].contract_name;
    const safeName = contractName.replace(/[^a-zA-Z0-9]/g, '').toLowerCase();
    scripts += `    ${safeName})
        deploy_${safeName}
        ;;
`;
  });
  
  scripts += `    all)
        deploy_all
        ;;
    *)
        usage
        exit 1
        ;;
esac
`;
  
  return scripts;
}

/**
 * Main execution
 */
function main() {
  console.log('📚 Generating contract documentation...');
  
  const manifests = loadManifests();
  
  if (Object.keys(manifests).length === 0) {
    console.error('❌ No manifests found!');
    process.exit(1);
  }
  
  console.log(`📋 Found ${Object.keys(manifests).length} manifests`);
  
  // Generate Markdown documentation
  console.log('📝 Generating Markdown documentation...');
  const markdown = generateMarkdown(manifests);
  fs.writeFileSync(path.join(OUTPUT_DIR, 'api-documentation.md'), markdown);
  
  // Generate TypeScript definitions
  console.log('🔷 Generating TypeScript definitions...');
  const types = generateTypeScript(manifests);
  fs.writeFileSync(path.join(OUTPUT_DIR, 'contracts.d.ts'), types);
  
  // Generate deployment scripts
  console.log('🚀 Generating deployment scripts...');
  const scripts = generateDeploymentScripts(manifests);
  fs.writeFileSync(path.join(OUTPUT_DIR, 'deploy.sh'), scripts);
  
  // Generate index file
  console.log('📚 Generating documentation index...');
  const index = `# Generated Contract Documentation

This directory contains automatically generated documentation from contract manifests.

## Files

- \`api-documentation.md\` - Complete API documentation in Markdown format
- \`contracts.d.ts\` - TypeScript type definitions for all contracts
- \`deploy.sh\` - Deployment scripts for all contracts

## Generation

These files were generated on: ${new Date().toISOString()}

To regenerate:
\`\`\`bash
node scripts/generate-docs.js
\`\`\`

## Source

Generated from the following manifest files:
${Object.keys(manifests).map(filename => `- \`${filename}\``).join('\n')}
`;
  fs.writeFileSync(path.join(OUTPUT_DIR, 'README.md'), index);
  
  console.log('✅ Documentation generation complete!');
  console.log(`📁 Output directory: ${OUTPUT_DIR}`);
  console.log('');
  console.log('Generated files:');
  console.log('  📝 api-documentation.md - Complete API documentation');
  console.log('  🔷 contracts.d.ts - TypeScript definitions');
  console.log('  🚀 deploy.sh - Deployment scripts');
  console.log('  📚 README.md - Documentation index');
}

// Run if called directly
if (require.main === module) {
  main();
}

module.exports = {
  loadManifests,
  generateMarkdown,
  generateTypeScript,
  generateDeploymentScripts
};
