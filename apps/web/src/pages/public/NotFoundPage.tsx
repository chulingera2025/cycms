import { Link } from 'react-router-dom';

export default function NotFoundPage() {
  return (
    <div className="not-found-page">
      <h1>404</h1>
      <p>抱歉，您访问的页面不存在</p>
      <Link to="/" className="btn btn-primary">返回首页</Link>
    </div>
  );
}
