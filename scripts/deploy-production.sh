#!/bin/bash

# Production Deployment Script
# This script deploys the tops-worker to the production environment with safety checks

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NAMESPACE="tops-worker-production"
KUSTOMIZE_PATH="k8s/overlays/production"
TIMEOUT=600
BACKUP_DIR="backups/$(date +%Y%m%d-%H%M%S)"

echo -e "${BLUE}🚀 Starting production deployment...${NC}"

# Safety check - require confirmation
echo -e "${RED}⚠️  WARNING: This will deploy to PRODUCTION environment${NC}"
echo -e "${YELLOW}Are you sure you want to continue? (yes/no)${NC}"
read -r confirmation

if [[ "$confirmation" != "yes" ]]; then
    echo -e "${YELLOW}Deployment cancelled.${NC}"
    exit 0
fi

# Check if kubectl is available
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}❌ kubectl is not installed or not in PATH${NC}"
    exit 1
fi

# Check if kustomize is available
if ! command -v kustomize &> /dev/null; then
    echo -e "${YELLOW}⚠️  kustomize not found, installing...${NC}"
    curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
    export PATH=$PATH:$(pwd)
fi

# Check cluster connectivity
echo -e "${BLUE}🔍 Checking cluster connectivity...${NC}"
if ! kubectl cluster-info &> /dev/null; then
    echo -e "${RED}❌ Cannot connect to Kubernetes cluster${NC}"
    exit 1
fi

# Verify we're on the correct cluster
echo -e "${BLUE}🔍 Verifying cluster context...${NC}"
CURRENT_CONTEXT=$(kubectl config current-context)
echo -e "${YELLOW}Current context: $CURRENT_CONTEXT${NC}"

# Safety check - confirm cluster
echo -e "${YELLOW}Is this the correct production cluster? (yes/no)${NC}"
read -r cluster_confirmation

if [[ "$cluster_confirmation" != "yes" ]]; then
    echo -e "${YELLOW}Deployment cancelled. Please switch to the correct cluster.${NC}"
    exit 0
fi

# Create backup directory
echo -e "${BLUE}💾 Creating backup...${NC}"
mkdir -p "$BACKUP_DIR"

# Backup current configuration
echo -e "${BLUE}   Backing up current configuration...${NC}"
kubectl get all -n "$NAMESPACE" -o yaml > "$BACKUP_DIR/current-deployment.yaml" 2>/dev/null || true
kubectl get configmap -n "$NAMESPACE" -o yaml > "$BACKUP_DIR/configmaps.yaml" 2>/dev/null || true
kubectl get secret -n "$NAMESPACE" -o yaml > "$BACKUP_DIR/secrets.yaml" 2>/dev/null || true

echo -e "${GREEN}✅ Backup created in $BACKUP_DIR${NC}"

# Create namespace if it doesn't exist
echo -e "${BLUE}📦 Creating namespace if needed...${NC}"
kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

# Pre-deployment health check
echo -e "${BLUE}🏥 Pre-deployment health check...${NC}"
if kubectl get pods -n "$NAMESPACE" &> /dev/null; then
    echo -e "${YELLOW}   Checking existing deployment health...${NC}"
    kubectl get pods -n "$NAMESPACE" -l app=tops-worker
fi

# Apply the production configuration
echo -e "${BLUE}🔧 Applying production configuration...${NC}"
kustomize build "$KUSTOMIZE_PATH" | kubectl apply -f -

# Wait for deployments to be ready
echo -e "${BLUE}⏳ Waiting for deployments to be ready...${NC}"

# Wait for worker deployment
echo -e "${BLUE}   Waiting for tops-worker deployment...${NC}"
kubectl rollout status deployment/tops-worker -n "$NAMESPACE" --timeout="${TIMEOUT}s"

# Wait for verifier deployment
echo -e "${BLUE}   Waiting for tops-worker-verifier deployment...${NC}"
kubectl rollout status deployment/tops-worker-verifier -n "$NAMESPACE" --timeout="${TIMEOUT}s"

# Check pod status
echo -e "${BLUE}🔍 Checking pod status...${NC}"
kubectl get pods -n "$NAMESPACE" -l app=tops-worker

# Verify all pods are running
POD_STATUS=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker --no-headers | awk '{print $3}')
if echo "$POD_STATUS" | grep -q -E "(Pending|CrashLoopBackOff|Error|Failed)"; then
    echo -e "${RED}❌ Some pods are not running properly${NC}"
    kubectl get pods -n "$NAMESPACE" -l app=tops-worker
    exit 1
fi

# Get service URLs
echo -e "${BLUE}🌐 Service endpoints:${NC}"
kubectl get svc -n "$NAMESPACE" -l app=tops-worker

# Run comprehensive health checks
echo -e "${BLUE}🏥 Running health checks...${NC}"

# Check worker health
WORKER_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=worker -o jsonpath='{.items[*].metadata.name}')
for pod in $WORKER_PODS; do
    if kubectl exec -n "$NAMESPACE" "$pod" -- curl -f http://localhost:8082/health &> /dev/null; then
        echo -e "${GREEN}✅ Worker pod $pod health check passed${NC}"
    else
        echo -e "${RED}❌ Worker pod $pod health check failed${NC}"
        exit 1
    fi
done

# Check verifier health
VERIFIER_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=verifier -o jsonpath='{.items[*].metadata.name}')
for pod in $VERIFIER_PODS; do
    if kubectl exec -n "$NAMESPACE" "$pod" -- curl -f http://localhost:8081/healthz &> /dev/null; then
        echo -e "${GREEN}✅ Verifier pod $pod health check passed${NC}"
    else
        echo -e "${RED}❌ Verifier pod $pod health check failed${NC}"
        exit 1
    fi
done

# Check metrics endpoints
echo -e "${BLUE}📊 Checking metrics endpoints...${NC}"
WORKER_POD=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=worker -o jsonpath='{.items[0].metadata.name}')
if kubectl exec -n "$NAMESPACE" "$WORKER_POD" -- curl -f http://localhost:8082/prometheus &> /dev/null; then
    echo -e "${GREEN}✅ Metrics endpoint accessible${NC}"
else
    echo -e "${YELLOW}⚠️  Metrics endpoint not accessible${NC}"
fi

# Load testing (optional)
echo -e "${BLUE}🧪 Running load test...${NC}"
echo -e "${YELLOW}   Sending test requests...${NC}"

# Port forward for testing
kubectl port-forward -n "$NAMESPACE" svc/tops-worker-service 8082:8082 &
PF_PID=$!
sleep 5

# Send test requests
for i in {1..5}; do
    if curl -f http://localhost:8082/health &> /dev/null; then
        echo -e "${GREEN}   Test request $i passed${NC}"
    else
        echo -e "${RED}   Test request $i failed${NC}"
        kill $PF_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

kill $PF_PID 2>/dev/null || true

# Show deployment summary
echo -e "${BLUE}📋 Deployment summary:${NC}"
echo -e "${YELLOW}Namespace:${NC} $NAMESPACE"
echo -e "${YELLOW}Worker replicas:${NC} $(kubectl get deployment tops-worker -n "$NAMESPACE" -o jsonpath='{.spec.replicas}')"
echo -e "${YELLOW}Verifier replicas:${NC} $(kubectl get deployment tops-worker-verifier -n "$NAMESPACE" -o jsonpath='{.spec.replicas}')"
echo -e "${YELLOW}Backup location:${NC} $BACKUP_DIR"

# Show recent logs
echo -e "${BLUE}📋 Recent logs:${NC}"
echo -e "${YELLOW}Worker logs:${NC}"
kubectl logs -n "$NAMESPACE" deployment/tops-worker --tail=5

echo -e "${YELLOW}Verifier logs:${NC}"
kubectl logs -n "$NAMESPACE" deployment/tops-worker-verifier --tail=5

echo -e "${GREEN}🎉 Production deployment completed successfully!${NC}"
echo -e "${BLUE}📊 Monitor the deployment:${NC}"
echo -e "   kubectl get pods -n $NAMESPACE -w"
echo -e "   kubectl logs -n $NAMESPACE -f deployment/tops-worker"
echo -e "   kubectl get hpa -n $NAMESPACE"
echo -e "${BLUE}🔄 Rollback if needed:${NC}"
echo -e "   kubectl apply -f $BACKUP_DIR/current-deployment.yaml"
