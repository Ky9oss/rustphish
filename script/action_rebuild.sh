
# 检查是否提供了参数
if [ $# -eq 0 ]; then
    echo "Usage: $0 <tag-name>"
    exit 1
fi

# 获取第一个参数作为标签名称
TAG_NAME=$1

# 打印标签名称
echo "Delete and ReCreating git tag: $TAG_NAME"

# 创建并推送标签
git add .
git commit -m "update"
proxychains git push -u origin main
git tag -d $TAG_NAME
git push --delete origin $TAG_NAME
git tag $TAG_NAME
git push origin $TAG_NAME