#!/bin/bash

# Staging Deployment Script
# This script deploys the tops-worker to the staging environment

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NAMESPACE="tops-worker-staging"
KUSTOMIZE_PATH="k8s/overlays/staging"
TIMEOUT=300

echo -e "${BLUE}🚀 Starting staging deployment...${NC}"

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

# Create namespace if it doesn't exist
echo -e "${BLUE}📦 Creating namespace if needed...${NC}"
kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

# Apply the staging configuration
echo -e "${BLUE}🔧 Applying staging configuration...${NC}"
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

# Get service URLs
echo -e "${BLUE}🌐 Service endpoints:${NC}"
kubectl get svc -n "$NAMESPACE" -l app=tops-worker

# Run health checks
echo -e "${BLUE}🏥 Running health checks...${NC}"

# Check worker health
WORKER_POD=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=worker -o jsonpath='{.items[0].metadata.name}')
if kubectl exec -n "$NAMESPACE" "$WORKER_POD" -- curl -f http://localhost:8082/health &> /dev/null; then
    echo -e "${GREEN}✅ Worker health check passed${NC}"
else
    echo -e "${RED}❌ Worker health check failed${NC}"
    exit 1
fi

# Check verifier health
VERIFIER_POD=$(kubectl get pods -n "$NAMESPACE" -l app=tops-worker,component=verifier -o jsonpath='{.items[0].metadata.name}')
if kubectl exec -n "$NAMESPACE" "$VERIFIER_POD" -- curl -f http://localhost:8081/healthz &> /dev/null; then
    echo -e "${GREEN}✅ Verifier health check passed${NC}"
else
    echo -e "${RED}❌ Verifier health check failed${NC}"
    exit 1
fi

# Show logs for verification
echo -e "${BLUE}📋 Recent logs:${NC}"
echo -e "${YELLOW}Worker logs:${NC}"
kubectl logs -n "$NAMESPACE" deployment/tops-worker --tail=10

echo -e "${YELLOW}Verifier logs:${NC}"
kubectl logs -n "$NAMESPACE" deployment/tops-worker-verifier --tail=10

echo -e "${GREEN}🎉 Staging deployment completed successfully!${NC}"
echo -e "${BLUE}📊 Monitor the deployment:${NC}"
echo -e "   kubectl get pods -n $NAMESPACE -w"
echo -e "   kubectl logs -n $NAMESPACE -f deployment/tops-worker"
