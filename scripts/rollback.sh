#!/bin/bash

# Rollback Script
# This script rolls back the tops-worker deployment to a previous version

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NAMESPACE="${1:-tops-worker-production}"
BACKUP_DIR="${2:-}"

echo -e "${BLUE}🔄 Starting rollback process...${NC}"

# Check if kubectl is available
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}❌ kubectl is not installed or not in PATH${NC}"
    exit 1
fi

# Check cluster connectivity
echo -e "${BLUE}🔍 Checking cluster connectivity...${NC}"
if ! kubectl cluster-info &> /dev/null; then
    echo -e "${RED}❌ Cannot connect to Kubernetes cluster${NC}"
    exit 1
fi

# List available backups if no backup directory specified
if [[ -z "$BACKUP_DIR" ]]; then
    echo -e "${BLUE}📋 Available backups:${NC}"
    if [[ -d "backups" ]]; then
        ls -la backups/ | grep "^d" | awk '{print $9}' | sort -r
        echo -e "${YELLOW}Please specify a backup directory:${NC}"
        echo -e "   ./scripts/rollback.sh $NAMESPACE backups/YYYYMMDD-HHMMSS"
        exit 1
    else
        echo -e "${RED}❌ No backups directory found${NC}"
        exit 1
    fi
fi

# Verify backup directory exists
if [[ ! -d "$BACKUP_DIR" ]]; then
    echo -e "${RED}❌ Backup directory $BACKUP_DIR does not exist${NC}"
    exit 1
fi

# Safety check - require confirmation
echo -e "${RED}⚠️  WARNING: This will rollback the deployment in namespace: $NAMESPACE${NC}"
echo -e "${YELLOW}Backup directory: $BACKUP_DIR${NC}"
echo -e "${YELLOW}Are you sure you want to continue? (yes/no)${NC}"
read -r confirmation

if [[ "$confirmation" != "yes" ]]; then
    echo -e "${YELLOW}Rollback cancelled.${NC}"
    exit 0
fi

# Check current deployment status
echo -e "${BLUE}🔍 Current deployment status:${NC}"
kubectl get pods -n "$NAMESPACE" -l app=tops-worker 2>/dev/null || echo -e "${YELLOW}No current deployment found${NC}"

# Create backup of current state before rollback
echo -e "${BLUE}💾 Creating backup of current state...${NC}"
CURRENT_BACKUP="backups/pre-rollback-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$CURRENT_BACKUP"

kubectl get all -n "$NAMESPACE" -o yaml > "$CURRENT_BACKUP/current-deployment.yaml" 2>/dev/null || true
kubectl get configmap -n "$NAMESPACE" -o yaml > "$CURRENT_BACKUP/configmaps.yaml" 2>/dev/null || true
kubectl get secret -n "$NAMESPACE" -o yaml > "$CURRENT_BACKUP/secrets.yaml" 2>/dev/null || true

echo -e "${GREEN}✅ Current state backed up to $CURRENT_BACKUP${NC}"

# Apply rollback configuration
echo -e "${BLUE}🔧 Applying rollback configuration...${NC}"

# Check what backup files are available
BACKUP_FILES=()
[[ -f "$BACKUP_DIR/current-deployment.yaml" ]] && BACKUP_FILES+=("$BACKUP_DIR/current-deployment.yaml")
[[ -f "$BACKUP_DIR/configmaps.yaml" ]] && BACKUP_FILES+=("$BACKUP_DIR/configmaps.yaml")
[[ -f "$BACKUP_DIR/secrets.yaml" ]] && BACKUP_FILES+=("$BACKUP_DIR/secrets.yaml")

if [[ ${#BACKUP_FILES[@]} -eq 0 ]]; then
    echo -e "${RED}❌ No valid backup files found in $BACKUP_DIR${NC}"
    exit 1
fi

# Apply backup files
for file in "${BACKUP_FILES[@]}"; do
    echo -e "${BLUE}   Applying $file...${NC}"
    kubectl apply -f "$file" --force
done

# Wait for rollback to complete
echo -e "${BLUE}⏳ Waiting for rollback to complete...${NC}"

# Wait for deployments to be ready
if kubectl get deployment tops-worker -n "$NAMESPACE" &> /dev/null; then
    echo -e "${BLUE}   Waiting for tops-worker deployment...${NC}"
    kubectl rollout status deployment/tops-worker -n "$NAMESPACE" --timeout=300s
fi

if kubectl get deployment tops-worker-verifier -n "$NAMESPACE" &> /dev/null; then
    echo -e "${BLUE}   Waiting for tops-worker-verifier deployment...${NC}"
    kubectl rollout status deployment/tops-worker-verifier -n "$NAMESPACE" --timeout=300s
fi

# Check pod status
echo -e "${BLUE}🔍 Checking pod status after rollback...${NC}"
kubectl get pods -n "$NAMESPACE" -l app=tops-worker

# Verify all pods are running
POD_STATUS=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker --no-headers 2>/dev/null | awk '{print $3}' || echo "")
if [[ -n "$POD_STATUS" ]] && echo "$POD_STATUS" | grep -q -E "(Pending|CrashLoopBackOff|Error|Failed)"; then
    echo -e "${RED}❌ Some pods are not running properly after rollback${NC}"
    kubectl get pods -n "$NAMESPACE" -l app=tops-worker
    exit 1
fi

# Run health checks
echo -e "${BLUE}🏥 Running health checks after rollback...${NC}"

# Check worker health
WORKER_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=worker -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
if [[ -n "$WORKER_PODS" ]]; then
    for pod in $WORKER_PODS; do
        if kubectl exec -n "$NAMESPACE" "$pod" -- curl -f http://localhost:8082/health &> /dev/null; then
            echo -e "${GREEN}✅ Worker pod $pod health check passed${NC}"
        else
            echo -e "${RED}❌ Worker pod $pod health check failed${NC}"
            exit 1
        fi
    done
fi

# Check verifier health
VERIFIER_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=verifier -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
if [[ -n "$VERIFIER_PODS" ]]; then
    for pod in $VERIFIER_PODS; do
        if kubectl exec -n "$NAMESPACE" "$pod" -- curl -f http://localhost:8081/healthz &> /dev/null; then
            echo -e "${GREEN}✅ Verifier pod $pod health check passed${NC}"
        else
            echo -e "${RED}❌ Verifier pod $pod health check failed${NC}"
            exit 1
        fi
    done
fi

# Show rollback summary
echo -e "${BLUE}📋 Rollback summary:${NC}"
echo -e "${YELLOW}Namespace:${NC} $NAMESPACE"
echo -e "${YELLOW}Rollback from:${NC} $BACKUP_DIR"
echo -e "${YELLOW}Current backup:${NC} $CURRENT_BACKUP"

# Show recent logs
echo -e "${BLUE}📋 Recent logs after rollback:${NC}"
if kubectl get deployment tops-worker -n "$NAMESPACE" &> /dev/null; then
    echo -e "${YELLOW}Worker logs:${NC}"
    kubectl logs -n "$NAMESPACE" deployment/tops-worker --tail=5
fi

if kubectl get deployment tops-worker-verifier -n "$NAMESPACE" &> /dev/null; then
    echo -e "${YELLOW}Verifier logs:${NC}"
    kubectl logs -n "$NAMESPACE" deployment/tops-worker-verifier --tail=5
fi

echo -e "${GREEN}🎉 Rollback completed successfully!${NC}"
echo -e "${BLUE}📊 Monitor the deployment:${NC}"
echo -e "   kubectl get pods -n $NAMESPACE -w"
echo -e "   kubectl logs -n $NAMESPACE -f deployment/tops-worker"
echo -e "${BLUE}📋 Rollback information:${NC}"
echo -e "   Rollback from: $BACKUP_DIR"
echo -e "   Current backup: $CURRENT_BACKUP"
