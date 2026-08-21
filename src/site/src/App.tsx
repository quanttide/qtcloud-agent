import "./App.css";

function App() {
  return (
    <div className="app">
      <header className="hero">
        <h1>量潮智能体云</h1>
        <p className="tagline">一个高效的蒸馏人类隐式经验的工具</p>
      </header>

      <section className="pain">
        <h2>你有没有发现</h2>
        <p>
          你每天都在教 AI 做事情——怎么处理你的工作、怎么按你的习惯来、
          怎么判断什么重要什么不重要。这些经验，是你花了很多年才积累起来的。
        </p>
        <p>
          但它们散落在一次次的对话里。用了一次，就再也没用过。
          你教过一次的事，下次还要重新教一遍。
        </p>
      </section>

      <section className="how">
        <h2>智能体云帮你把这些经验留下来</h2>
        <div className="steps">
          <div className="step">
            <h3>分析对话</h3>
            <p>把你和智能体的对话导入进来，标注它做了什么、怎么做的。</p>
          </div>
          <div className="step">
            <h3>制作技能</h3>
            <p>从对话中提取可复用的技能资产——skill、上下文、知识——让它们在任何地方都能用。</p>
          </div>
          <div className="step">
            <h3>治理观测</h3>
            <p>持续观察你的智能体，知道哪个好用、哪个该改进，让经验不断升级。</p>
          </div>
        </div>
      </section>

      <footer>
        <p>量潮智能体云 · 正在建设中</p>
      </footer>
    </div>
  );
}

export default App;
